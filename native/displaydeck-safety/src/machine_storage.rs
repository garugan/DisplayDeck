#[cfg(target_os = "windows")]
mod platform {
    use std::{mem::size_of, ptr};

    use windows::{
        core::{PCWSTR, PWSTR},
        Wdk::{
            Foundation::OBJECT_ATTRIBUTES,
            Storage::FileSystem::{
                NtCreateFile, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
                FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
            },
        },
        Win32::{
            Foundation::{
                CloseHandle, HANDLE, HLOCAL, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, UNICODE_STRING,
            },
            Security::Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
            Security::{
                AclSizeInformation, CreateWellKnownSid, EqualSid, GetAce, GetAclInformation,
                GetSecurityDescriptorControl, GetTokenInformation, IsValidSid, TokenUser,
                WinBuiltinAdministratorsSid, WinLocalSystemSid, ACCESS_ALLOWED_ACE, ACE_HEADER,
                ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
                PSECURITY_DESCRIPTOR, PSID, SE_DACL_PRESENT, SE_DACL_PROTECTED, SE_SELF_RELATIVE,
                TOKEN_QUERY, TOKEN_USER,
            },
            Storage::FileSystem::{
                FileAttributeTagInfo, FileIdInfo, FileStandardInfo, FileStreamInfo, GetDriveTypeW,
                GetFileInformationByHandleEx, GetFinalPathNameByHandleW,
                GetVolumeInformationByHandleW, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_COMPRESSED,
                FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_HIDDEN,
                FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_NOT_CONTENT_INDEXED, FILE_ATTRIBUTE_OFFLINE,
                FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_SYSTEM,
                FILE_ATTRIBUTE_TAG_INFO, FILE_ID_INFO, FILE_READ_ATTRIBUTES, FILE_READ_DATA,
                FILE_READ_EA, FILE_SHARE_READ, FILE_STANDARD_INFO, FILE_STREAM_INFO, FILE_TRAVERSE,
                FILE_WRITE_DATA, READ_CONTROL, SYNCHRONIZE, VOLUME_NAME_GUID,
            },
            System::{
                Com::CoTaskMemFree,
                SystemServices::{
                    FILE_NAMED_STREAMS, FILE_PERSISTENT_ACLS, FILE_SUPPORTS_HARD_LINKS,
                },
                Threading::{GetCurrentProcess, OpenProcessToken},
                IO::IO_STATUS_BLOCK,
            },
            UI::Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath, KF_FLAG_DEFAULT},
        },
    };

    use super::{D07Anchor, D07StorageFailure, D07StorageVerdict};

    const DIRECTORY: &str = "DisplayDeck";
    const ACTOR: &str = "MachineActorRecordV1";
    const PROVISION: &str = "MachineActorProvisionRecordV1";
    const SYSTEM_FULL: u32 = 0x001f_01ff;
    const ADMIN_READ: u32 = 0x0012_0089;
    const DIRECTORY_TRAVERSE: u32 = 0x0012_00a8;
    const RECORD_SLOT_WRITE: u32 = 0x0012_008b;
    const DRIVE_FIXED: u32 = 3;
    const PROVISION_LENGTH: i64 = 12_288;
    const ACTOR_LENGTH: i64 = 135_168;

    pub(super) fn inspect() -> D07StorageVerdict {
        let Ok(program_data) = known_program_data() else {
            return no_go(D07StorageFailure::ProgramDataUnavailable);
        };
        let Some(token_sid) = token_sid() else {
            return no_go(D07StorageFailure::DaclUnproven);
        };
        let Some(root) = open_absolute_directory(&program_data) else {
            return no_go(D07StorageFailure::DirectoryAnchorUnproven);
        };
        let Some(root_evidence) = verify_root(root.0) else {
            return no_go(D07StorageFailure::DirectoryAnchorUnproven);
        };
        let Some(volume) = volume_profile(root.0) else {
            return no_go(D07StorageFailure::LocalFixedNtfsUnproven);
        };
        if root_evidence.id.VolumeSerialNumber != u64::from(volume.serial)
            || root_evidence.final_path != volume.root_path
        {
            return no_go(D07StorageFailure::LocalFixedNtfsUnproven);
        }
        let Some(directory) = open_relative_directory(root.0, DIRECTORY) else {
            return no_go(D07StorageFailure::MissingComponent);
        };
        let Some(directory_evidence) = verify_directory(directory.0, &token_sid) else {
            return no_go(D07StorageFailure::DirectoryAnchorUnproven);
        };
        if !is_direct_child(
            &root_evidence.final_path,
            &directory_evidence.final_path,
            DIRECTORY,
        ) {
            return no_go(D07StorageFailure::DirectoryAnchorUnproven);
        }
        let Some(provision) = open_relative_file(directory.0, PROVISION, false) else {
            return no_go(D07StorageFailure::MissingComponent);
        };
        let Some(provision_evidence) =
            verify_file(provision.0, &token_sid, ADMIN_READ, PROVISION_LENGTH)
        else {
            return no_go(D07StorageFailure::DaclUnproven);
        };
        if !is_direct_child(
            &directory_evidence.final_path,
            &provision_evidence.final_path,
            PROVISION,
        ) {
            return no_go(D07StorageFailure::DirectoryAnchorUnproven);
        }
        let Some(actor) = open_relative_file(directory.0, ACTOR, true) else {
            return no_go(D07StorageFailure::MissingComponent);
        };
        let Some(actor_evidence) =
            verify_file(actor.0, &token_sid, RECORD_SLOT_WRITE, ACTOR_LENGTH)
        else {
            return no_go(D07StorageFailure::DaclUnproven);
        };
        if !is_direct_child(
            &directory_evidence.final_path,
            &actor_evidence.final_path,
            ACTOR,
        ) || root_evidence.id.VolumeSerialNumber != directory_evidence.id.VolumeSerialNumber
            || root_evidence.id.VolumeSerialNumber != provision_evidence.id.VolumeSerialNumber
            || root_evidence.id.VolumeSerialNumber != actor_evidence.id.VolumeSerialNumber
        {
            return no_go(D07StorageFailure::DirectoryAnchorUnproven);
        }
        D07StorageVerdict::Go(D07Anchor {
            root,
            directory,
            provision,
            actor,
            token_sid,
            root_evidence,
            directory_evidence,
            provision_evidence,
            actor_evidence,
            volume,
        })
    }

    fn no_go(reason: D07StorageFailure) -> D07StorageVerdict {
        D07StorageVerdict::NoGo(reason)
    }

    fn known_program_data() -> Result<Vec<u16>, ()> {
        // SAFETY: the shell returns a CoTaskMem-allocated, NUL-terminated UTF-16 string.
        let raw = unsafe { SHGetKnownFolderPath(&FOLDERID_ProgramData, KF_FLAG_DEFAULT, None) }
            .map_err(|_| ())?;
        if raw.0.is_null() {
            return Err(());
        }
        let mut length = 0usize;
        // SAFETY: `raw` remains valid until CoTaskMemFree below.
        while length < 32_768 && unsafe { *raw.0.add(length) } != 0 {
            length += 1;
        }
        let result = if length == 32_768 {
            Err(())
        } else {
            // SAFETY: valid UTF-16 buffer owned by the shell; copy before freeing it.
            Ok(unsafe { std::slice::from_raw_parts(raw.0, length) }.to_vec())
        };
        // SAFETY: matching allocator for SHGetKnownFolderPath.
        unsafe { CoTaskMemFree(Some(raw.0.cast())) };
        let mut path = result?;
        if path.is_empty() || path.starts_with(&['\\' as u16, '\\' as u16]) {
            return Err(());
        }
        path.push(0);
        Ok(path)
    }

    #[derive(Clone, Debug, PartialEq)]
    pub(super) struct VolumeProfile {
        serial: u32,
        flags: u32,
        guid_root: Vec<u16>,
        root_path: Vec<u16>,
    }

    pub(super) fn volume_profile(handle: HANDLE) -> Option<VolumeProfile> {
        let mut fs = [0_u16; 64];
        let mut flags = 0_u32;
        let mut serial = 0_u32;
        // SAFETY: all buffers are valid for the handle-bound volume query.
        unsafe {
            GetVolumeInformationByHandleW(
                handle,
                None,
                Some(&mut serial),
                None,
                Some(&mut flags),
                Some(&mut fs),
            )
        }
        .ok()?;
        let required_flags = FILE_PERSISTENT_ACLS | FILE_NAMED_STREAMS | FILE_SUPPORTS_HARD_LINKS;
        if serial == 0
            || flags & required_flags != required_flags
            || !fs.starts_with(&['N' as u16, 'T' as u16, 'F' as u16, 'S' as u16, 0])
        {
            return None;
        }
        let mut path = [0_u16; 32_768];
        // SAFETY: returned UTF-16 path is bounded by the fixed output buffer.
        let length =
            unsafe { GetFinalPathNameByHandleW(handle, &mut path, VOLUME_NAME_GUID) } as usize;
        if length == 0 || length >= path.len() {
            return None;
        }
        let root_path = normalize_final_path(&path[..length])?;
        let mut guid_root = volume_guid_root(&root_path)?;
        guid_root.push(0);
        // SAFETY: `guid_root` is the NUL-terminated volume GUID root derived from this handle.
        if unsafe { GetDriveTypeW(PCWSTR(guid_root.as_ptr())) } != DRIVE_FIXED {
            return None;
        }
        guid_root.pop();
        Some(VolumeProfile {
            serial,
            flags,
            guid_root,
            root_path,
        })
    }

    fn volume_guid_root(path: &[u16]) -> Option<Vec<u16>> {
        let prefix = r"\\?\volume{".encode_utf16().collect::<Vec<_>>();
        if !path.starts_with(&prefix) {
            return None;
        }
        let guid_start = prefix.len();
        let guid_end = guid_start.checked_add(36)?;
        let root_end = guid_end.checked_add(2)?;
        if path.get(guid_end) != Some(&('}' as u16))
            || path.get(guid_end + 1) != Some(&('\\' as u16))
            || !path
                .get(guid_start..guid_end)?
                .iter()
                .enumerate()
                .all(|(index, unit)| {
                    if matches!(index, 8 | 13 | 18 | 23) {
                        *unit == '-' as u16
                    } else {
                        *unit <= u16::from(u8::MAX)
                            && matches!(*unit as u8, b'0'..=b'9' | b'a'..=b'f')
                    }
                })
        {
            return None;
        }
        Some(path.get(..root_end)?.to_vec())
    }

    fn open_absolute_directory(program_data: &[u16]) -> Option<HeldHandle> {
        let path = program_data.strip_suffix(&[0])?;
        if path.len() < 3 || path[1] != ':' as u16 || path[2] != '\\' as u16 {
            return None;
        }
        let mut nt_path = "\\??\\".encode_utf16().collect::<Vec<_>>();
        nt_path.extend_from_slice(path);
        nt_path.push(0);
        open(None, &nt_path, true, false)
    }
    fn open_relative_directory(root: HANDLE, name: &str) -> Option<HeldHandle> {
        let mut name = name.encode_utf16().collect::<Vec<_>>();
        name.push(0);
        open(Some(root), &name, true, false)
    }
    fn open_relative_file(root: HANDLE, name: &str, writable: bool) -> Option<HeldHandle> {
        let mut name = name.encode_utf16().collect::<Vec<_>>();
        name.push(0);
        open(Some(root), &name, false, writable)
    }
    fn open(
        root: Option<HANDLE>,
        name: &[u16],
        directory: bool,
        writable: bool,
    ) -> Option<HeldHandle> {
        let bytes = name.len().checked_sub(1)?.checked_mul(2)?;
        let length = u16::try_from(bytes).ok()?;
        let unicode = UNICODE_STRING {
            Length: length,
            MaximumLength: length,
            Buffer: PWSTR(name.as_ptr().cast_mut()),
        };
        let attributes = OBJECT_ATTRIBUTES {
            Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
            RootDirectory: root.unwrap_or_default(),
            ObjectName: &unicode,
            Attributes: OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE,
            SecurityDescriptor: ptr::null(),
            SecurityQualityOfService: ptr::null(),
        };
        let access = if directory {
            FILE_TRAVERSE | FILE_READ_EA | FILE_READ_ATTRIBUTES | READ_CONTROL | SYNCHRONIZE
        } else if writable {
            FILE_READ_DATA
                | FILE_WRITE_DATA
                | FILE_READ_EA
                | FILE_READ_ATTRIBUTES
                | READ_CONTROL
                | SYNCHRONIZE
        } else {
            FILE_READ_DATA | FILE_READ_EA | FILE_READ_ATTRIBUTES | READ_CONTROL | SYNCHRONIZE
        };
        let options = if directory {
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT
        } else {
            FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT
        };
        let mut handle = HANDLE::default();
        let mut io = IO_STATUS_BLOCK::default();
        // SAFETY: FILE_OPEN cannot create; name and attributes live through this synchronous call.
        let status = unsafe {
            NtCreateFile(
                &mut handle,
                access,
                &attributes,
                &mut io,
                None,
                Default::default(),
                if directory {
                    FILE_SHARE_READ
                } else {
                    Default::default()
                },
                FILE_OPEN,
                options,
                None,
                0,
            )
        };
        status.is_ok().then_some(HeldHandle(handle))
    }

    #[derive(Clone, Debug, PartialEq)]
    pub(super) struct ObjectEvidence {
        id: FILE_ID_INFO,
        attributes: u32,
        stream_name: Vec<u16>,
        final_path: Vec<u16>,
    }

    #[derive(Clone, Copy)]
    enum ObjectKind {
        Root,
        Directory,
        File { expected_length: i64 },
    }

    pub(super) fn verify_root(handle: HANDLE) -> Option<ObjectEvidence> {
        verify_identity(handle, ObjectKind::Root)
    }

    pub(super) fn verify_directory(handle: HANDLE, sid: &TokenSid) -> Option<ObjectEvidence> {
        dacl_matches(handle, sid, DIRECTORY_TRAVERSE)
            .then(|| verify_identity(handle, ObjectKind::Directory))?
    }

    pub(super) fn verify_file(
        handle: HANDLE,
        sid: &TokenSid,
        mask: u32,
        length: i64,
    ) -> Option<ObjectEvidence> {
        dacl_matches(handle, sid, mask).then(|| {
            verify_identity(
                handle,
                ObjectKind::File {
                    expected_length: length,
                },
            )
        })?
    }

    fn verify_identity(handle: HANDLE, kind: ObjectKind) -> Option<ObjectEvidence> {
        let mut tag = FILE_ATTRIBUTE_TAG_INFO::default();
        // SAFETY: `tag` is the exact FileAttributeTagInfo output type.
        unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileAttributeTagInfo,
                (&mut tag as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
                size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
            )
        }
        .ok()?;
        if !attributes_allowed(tag.FileAttributes, tag.ReparseTag, kind) {
            return None;
        }

        let stream_name = match kind {
            ObjectKind::Root => Vec::new(),
            ObjectKind::Directory | ObjectKind::File { .. } => {
                let name = single_stream_name(handle)?;
                if !stream_allowed(&name, kind) {
                    return None;
                }
                name
            }
        };
        if !matches!(kind, ObjectKind::Root) {
            let mut standard = FILE_STANDARD_INFO::default();
            // SAFETY: `standard` is the exact FileStandardInfo output type.
            unsafe {
                GetFileInformationByHandleEx(
                    handle,
                    FileStandardInfo,
                    (&mut standard as *mut FILE_STANDARD_INFO).cast(),
                    size_of::<FILE_STANDARD_INFO>() as u32,
                )
            }
            .ok()?;
            if standard.NumberOfLinks != 1
                || standard.DeletePending
                || standard.Directory != matches!(kind, ObjectKind::Directory)
                || matches!(kind, ObjectKind::File { expected_length } if standard.EndOfFile != expected_length)
            {
                return None;
            }
        }
        let mut id = FILE_ID_INFO::default();
        // SAFETY: `id` is the exact FileIdInfo output type.
        unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileIdInfo,
                (&mut id as *mut FILE_ID_INFO).cast(),
                size_of::<FILE_ID_INFO>() as u32,
            )
        }
        .ok()?;
        if id.VolumeSerialNumber == 0 || id.FileId.Identifier == [0; 16] {
            return None;
        }
        Some(ObjectEvidence {
            id,
            attributes: tag.FileAttributes,
            stream_name,
            final_path: final_path(handle)?,
        })
    }

    fn attributes_allowed(attributes: u32, reparse_tag: u32, kind: ObjectKind) -> bool {
        if reparse_tag != 0
            || attributes
                & (FILE_ATTRIBUTE_REPARSE_POINT.0
                    | FILE_ATTRIBUTE_COMPRESSED.0
                    | FILE_ATTRIBUTE_ENCRYPTED.0
                    | FILE_ATTRIBUTE_OFFLINE.0)
                != 0
        {
            return false;
        }
        let allowed = match kind {
            ObjectKind::Root => {
                FILE_ATTRIBUTE_DIRECTORY.0
                    | FILE_ATTRIBUTE_ARCHIVE.0
                    | FILE_ATTRIBUTE_HIDDEN.0
                    | FILE_ATTRIBUTE_SYSTEM.0
                    | FILE_ATTRIBUTE_READONLY.0
                    | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED.0
            }
            ObjectKind::Directory => {
                FILE_ATTRIBUTE_DIRECTORY.0
                    | FILE_ATTRIBUTE_ARCHIVE.0
                    | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED.0
            }
            ObjectKind::File { .. } => {
                FILE_ATTRIBUTE_ARCHIVE.0
                    | FILE_ATTRIBUTE_NORMAL.0
                    | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED.0
            }
        };
        attributes & !allowed == 0
            && match kind {
                ObjectKind::Root | ObjectKind::Directory => {
                    attributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0
                        && attributes & FILE_ATTRIBUTE_NORMAL.0 == 0
                }
                ObjectKind::File { .. } => {
                    attributes & FILE_ATTRIBUTE_DIRECTORY.0 == 0
                        && attributes != 0
                        && (attributes & FILE_ATTRIBUTE_NORMAL.0 == 0
                            || attributes == FILE_ATTRIBUTE_NORMAL.0)
                }
            }
    }

    fn single_stream_name(handle: HANDLE) -> Option<Vec<u16>> {
        let mut storage = [0_u64; 1_024];
        // SAFETY: the buffer is 8-byte aligned as required by FILE_STREAM_INFO.
        unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileStreamInfo,
                storage.as_mut_ptr().cast(),
                u32::try_from(std::mem::size_of_val(&storage)).ok()?,
            )
        }
        .ok()?;
        // SAFETY: a successful query writes the first aligned FILE_STREAM_INFO here.
        let stream = unsafe { &*storage.as_ptr().cast::<FILE_STREAM_INFO>() };
        let name_offset = std::mem::offset_of!(FILE_STREAM_INFO, StreamName);
        let name_length = usize::try_from(stream.StreamNameLength).ok()?;
        if stream.NextEntryOffset != 0
            || name_length % size_of::<u16>() != 0
            || name_offset.checked_add(name_length)? > std::mem::size_of_val(&storage)
        {
            return None;
        }
        // SAFETY: the offset is the real aligned C field and bounds were checked above.
        Some(
            unsafe {
                std::slice::from_raw_parts(
                    storage.as_ptr().cast::<u8>().add(name_offset).cast::<u16>(),
                    name_length / size_of::<u16>(),
                )
            }
            .to_vec(),
        )
    }

    fn stream_allowed(name: &[u16], kind: ObjectKind) -> bool {
        let value = String::from_utf16(name).ok();
        match kind {
            ObjectKind::Root => false,
            ObjectKind::Directory => matches!(
                value.as_deref(),
                Some("" | "::$DATA" | ":$I30:$INDEX_ALLOCATION" | "::$INDEX_ALLOCATION")
            ),
            ObjectKind::File { .. } => matches!(value.as_deref(), Some("" | "::$DATA")),
        }
    }

    fn final_path(handle: HANDLE) -> Option<Vec<u16>> {
        let mut path = [0_u16; 32_768];
        // SAFETY: the output buffer is writable and bounded.
        let length =
            unsafe { GetFinalPathNameByHandleW(handle, &mut path, VOLUME_NAME_GUID) } as usize;
        if length == 0 || length >= path.len() {
            return None;
        }
        normalize_final_path(&path[..length])
    }

    fn normalize_final_path(path: &[u16]) -> Option<Vec<u16>> {
        let text = String::from_utf16(path).ok()?.to_lowercase();
        (!text.contains('\0')).then(|| text.encode_utf16().collect())
    }

    fn is_direct_child(parent: &[u16], child: &[u16], name: &str) -> bool {
        let mut expected = parent.to_vec();
        if expected.last() != Some(&(b'\\' as u16)) {
            expected.push(b'\\' as u16);
        }
        expected.extend(name.to_lowercase().encode_utf16());
        child == expected
    }

    fn dacl_matches(handle: HANDLE, runtime: &TokenSid, runtime_mask: u32) -> bool {
        let mut owner = PSID::default();
        let mut dacl = ptr::null_mut();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: descriptor is released exactly once below.
        let result = unsafe {
            GetSecurityInfo(
                handle,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                Some(&mut owner),
                None,
                Some(&mut dacl),
                None,
                Some(&mut descriptor),
            )
        };
        if result.0 != 0 || owner.0.is_null() || dacl.is_null() || descriptor.0.is_null() {
            if !descriptor.0.is_null() {
                // SAFETY: a partial GetSecurityInfo result still uses LocalAlloc ownership.
                unsafe { windows::Win32::Foundation::LocalFree(Some(HLOCAL(descriptor.0))) };
            }
            return false;
        }
        let verdict = (|| {
            let system = well_known_sid(WinLocalSystemSid)?;
            let admins = well_known_sid(WinBuiltinAdministratorsSid)?;
            // SAFETY: pointers originate from successful security APIs and stay valid here.
            if !unsafe { IsValidSid(owner).as_bool() }
                || !unsafe { IsValidSid(runtime.as_psid()).as_bool() }
                || unsafe { EqualSid(owner, system.as_psid()) }.is_err()
                || unsafe { EqualSid(runtime.as_psid(), system.as_psid()) }.is_ok()
                || unsafe { EqualSid(runtime.as_psid(), admins.as_psid()) }.is_ok()
            {
                return None;
            }
            let mut control = 0u16;
            let mut revision = 0u32;
            unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) }
                .ok()?;
            if revision != 1
                || control != SE_DACL_PRESENT.0 | SE_DACL_PROTECTED.0 | SE_SELF_RELATIVE.0
            {
                return None;
            }
            let mut info = ACL_SIZE_INFORMATION::default();
            unsafe {
                GetAclInformation(
                    dacl,
                    (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
                    size_of::<ACL_SIZE_INFORMATION>() as u32,
                    AclSizeInformation,
                )
            }
            .ok()?;
            if info.AceCount != 3 || info.AclBytesInUse < 8 || info.AclBytesFree != 0 {
                return None;
            }
            let expected = [
                (system.as_psid(), SYSTEM_FULL),
                (admins.as_psid(), ADMIN_READ),
                (runtime.as_psid(), runtime_mask),
            ];
            let acl_bytes = usize::try_from(info.AclBytesInUse).ok()?;
            let sid_offset = std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart);
            let mut last_ace_end = 8usize;
            for (index, (expected_sid, expected_mask)) in expected.into_iter().enumerate() {
                let mut ace = ptr::null_mut();
                unsafe { GetAce(dacl, index as u32, &mut ace) }.ok()?;
                let ace_offset = (ace as usize).checked_sub(dacl as usize)?;
                if ace_offset != last_ace_end
                    || ace_offset.checked_add(size_of::<ACE_HEADER>())? > acl_bytes
                {
                    return None;
                }
                // SAFETY: GetAce returned a pointer inside the DACL and the header is in bounds.
                let header = unsafe { ptr::read_unaligned(ace.cast::<ACE_HEADER>()) };
                let ace_size = usize::from(header.AceSize);
                if header.AceType != 0
                    || header.AceFlags != 0
                    || ace_size < sid_offset + 8
                    || ace_size.checked_sub(sid_offset)? > 68
                    || ace_offset.checked_add(ace_size)? > acl_bytes
                {
                    return None;
                }
                // SAFETY: the complete fixed prefix is inside the already-bounded ACE.
                let allowed = unsafe { ptr::read_unaligned(ace.cast::<ACCESS_ALLOWED_ACE>()) };
                let mut sid = AlignedSid([0; 68]);
                let sid_bytes = ace_size.checked_sub(sid_offset)?;
                // SAFETY: source is inside the bounded ACE and destination has 68 bytes.
                unsafe {
                    ptr::copy_nonoverlapping(
                        ace.cast::<u8>().add(sid_offset),
                        sid.0.as_mut_ptr(),
                        sid_bytes,
                    )
                };
                let ace_sid = PSID(sid.0.as_mut_ptr().cast());
                let sid_length = unsafe { IsValidSid(ace_sid).as_bool() }
                    .then(|| unsafe { windows::Win32::Security::GetLengthSid(ace_sid) as usize })?;
                if sid_length != sid_bytes
                    || allowed.Mask != expected_mask
                    || unsafe { EqualSid(ace_sid, expected_sid) }.is_err()
                {
                    return None;
                }
                last_ace_end = ace_offset.checked_add(ace_size)?;
            }
            (last_ace_end == acl_bytes).then_some(())
        })()
        .is_some();
        // SAFETY: GetSecurityInfo returns a LocalAlloc-owned descriptor.
        unsafe { windows::Win32::Foundation::LocalFree(Some(HLOCAL(descriptor.0))) };
        verdict
    }

    fn well_known_sid(kind: windows::Win32::Security::WELL_KNOWN_SID_TYPE) -> Option<SidBuffer> {
        let mut storage = AlignedSid([0; 68]);
        let mut length = storage.0.len() as u32;
        // SAFETY: 68 bytes is the maximum Windows SID capacity for this DACL profile.
        unsafe {
            CreateWellKnownSid(
                kind,
                None,
                Some(PSID(storage.0.as_mut_ptr().cast())),
                &mut length,
            )
        }
        .ok()?;
        (usize::try_from(length).ok()? <= storage.0.len()
            && unsafe { IsValidSid(PSID(storage.0.as_mut_ptr().cast())).as_bool() })
        .then_some(SidBuffer { storage })
    }
    fn token_sid() -> Option<TokenSid> {
        let mut token = HANDLE::default();
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.ok()?;
        let result = (|| {
            let mut needed = 0u32;
            let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut needed) };
            if needed < size_of::<TOKEN_USER>() as u32 {
                return None;
            }
            let needed = usize::try_from(needed).ok()?;
            let mut storage = vec![0_u64; needed.div_ceil(size_of::<u64>())];
            let mut returned = u32::try_from(needed).ok()?;
            unsafe {
                GetTokenInformation(
                    token,
                    TokenUser,
                    Some(storage.as_mut_ptr().cast()),
                    u32::try_from(needed).ok()?,
                    &mut returned,
                )
            }
            .ok()?;
            let returned = usize::try_from(returned).ok()?;
            if returned < size_of::<TOKEN_USER>() || returned > needed {
                return None;
            }
            // SAFETY: storage is pointer-aligned and contains a returned TOKEN_USER prefix.
            let user = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() };
            let base = storage.as_ptr() as usize;
            let offset = (user.User.Sid.0 as usize).checked_sub(base)?;
            let available = returned.checked_sub(offset)?;
            if !(8..=68).contains(&available) {
                return None;
            }
            let mut sid = AlignedSid([0; 68]);
            // SAFETY: the source range was proven inside the returned token buffer.
            unsafe {
                ptr::copy_nonoverlapping(
                    storage.as_ptr().cast::<u8>().add(offset),
                    sid.0.as_mut_ptr(),
                    available,
                )
            };
            let sid_pointer = PSID(sid.0.as_mut_ptr().cast());
            if !unsafe { IsValidSid(sid_pointer).as_bool() }
                || unsafe { windows::Win32::Security::GetLengthSid(sid_pointer) as usize }
                    > available
            {
                return None;
            }
            Some(TokenSid { storage: sid })
        })();
        unsafe { CloseHandle(token) }.ok()?;
        result
    }
    pub(super) struct HeldHandle(pub(super) HANDLE);
    impl Drop for HeldHandle {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
    #[repr(align(8))]
    struct AlignedSid([u8; 68]);
    pub(super) struct SidBuffer {
        storage: AlignedSid,
    }
    impl SidBuffer {
        fn as_psid(&self) -> PSID {
            PSID(self.storage.0.as_ptr().cast_mut().cast())
        }
    }
    pub(super) struct TokenSid {
        storage: AlignedSid,
    }
    impl TokenSid {
        fn as_psid(&self) -> PSID {
            PSID(self.storage.0.as_ptr().cast_mut().cast())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn d07_path_stream_and_attribute_allowlists_are_exact() {
            let root = r"\\?\volume{01234567-89ab-cdef-0123-456789abcdef}\programdata"
                .encode_utf16()
                .collect::<Vec<_>>();
            assert_eq!(
                String::from_utf16(&volume_guid_root(&root).unwrap()).unwrap(),
                r"\\?\volume{01234567-89ab-cdef-0123-456789abcdef}\"
            );
            let child = r"\\?\volume{01234567-89ab-cdef-0123-456789abcdef}\programdata\displaydeck"
                .encode_utf16()
                .collect::<Vec<_>>();
            assert!(is_direct_child(&root, &child, "DisplayDeck"));
            assert!(stream_allowed(
                &"::$DATA".encode_utf16().collect::<Vec<_>>(),
                ObjectKind::File {
                    expected_length: ACTOR_LENGTH
                }
            ));
            assert!(stream_allowed(
                &":$I30:$INDEX_ALLOCATION".encode_utf16().collect::<Vec<_>>(),
                ObjectKind::Directory
            ));
            assert!(stream_allowed(&[], ObjectKind::Directory));
            assert!(stream_allowed(
                &[],
                ObjectKind::File {
                    expected_length: ACTOR_LENGTH
                }
            ));
            assert!(!stream_allowed(
                &":named:$DATA".encode_utf16().collect::<Vec<_>>(),
                ObjectKind::File {
                    expected_length: ACTOR_LENGTH
                }
            ));
            assert!(volume_guid_root(
                &r"\\?\volume{01234567-89ab-cdef-0123-456789abcdeg}\programdata"
                    .encode_utf16()
                    .collect::<Vec<_>>()
            )
            .is_none());
            assert!(volume_guid_root(
                &r"\\?\volume{01234567-89ab-cdef-0123-456789abcdef0}\programdata"
                    .encode_utf16()
                    .collect::<Vec<_>>()
            )
            .is_none());
            assert!(!attributes_allowed(
                FILE_ATTRIBUTE_ARCHIVE.0 | 0x200,
                0,
                ObjectKind::File {
                    expected_length: ACTOR_LENGTH
                }
            ));
        }
    }
}

#[cfg(target_os = "windows")]
pub struct D07Anchor {
    root: platform::HeldHandle,
    directory: platform::HeldHandle,
    provision: platform::HeldHandle,
    actor: platform::HeldHandle,
    token_sid: platform::TokenSid,
    root_evidence: platform::ObjectEvidence,
    directory_evidence: platform::ObjectEvidence,
    provision_evidence: platform::ObjectEvidence,
    actor_evidence: platform::ObjectEvidence,
    volume: platform::VolumeProfile,
}
#[cfg(target_os = "windows")]
impl D07Anchor {
    /// The retained handle closes the post-D07 replacement race; write code must revalidate it first.
    pub fn revalidate_before_actor_write(&self) -> bool {
        platform::volume_profile(self.root.0).as_ref() == Some(&self.volume)
            && platform::verify_root(self.root.0).as_ref() == Some(&self.root_evidence)
            && platform::verify_directory(self.directory.0, &self.token_sid).as_ref()
                == Some(&self.directory_evidence)
            && platform::verify_file(self.provision.0, &self.token_sid, 0x0012_0089, 12_288)
                .as_ref()
                == Some(&self.provision_evidence)
            && platform::verify_file(self.actor.0, &self.token_sid, 0x0012_008b, 135_168).as_ref()
                == Some(&self.actor_evidence)
    }
}
#[cfg(not(target_os = "windows"))]
pub struct D07Anchor;
#[cfg(not(target_os = "windows"))]
impl D07Anchor {
    pub fn revalidate_before_actor_write(&self) -> bool {
        false
    }
}
pub enum D07StorageVerdict {
    Go(D07Anchor),
    NoGo(D07StorageFailure),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum D07StorageFailure {
    NotWindows,
    ProgramDataUnavailable,
    LocalFixedNtfsUnproven,
    MissingComponent,
    DirectoryAnchorUnproven,
    DaclUnproven,
}
#[cfg(target_os = "windows")]
pub fn inspect_machine_actor_storage() -> D07StorageVerdict {
    platform::inspect()
}
#[cfg(not(target_os = "windows"))]
pub fn inspect_machine_actor_storage() -> D07StorageVerdict {
    D07StorageVerdict::NoGo(D07StorageFailure::NotWindows)
}
#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;
    #[test]
    fn non_windows_storage_is_always_no_go() {
        #[cfg(not(target_os = "windows"))]
        assert!(matches!(
            inspect_machine_actor_storage(),
            D07StorageVerdict::NoGo(D07StorageFailure::NotWindows)
        ));
    }
}
