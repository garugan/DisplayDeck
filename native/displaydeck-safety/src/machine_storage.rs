// WaitForSingleObject outcomes: Some(false) still owns an abandoned mutex.
#[cfg(any(target_os = "windows", test))]
fn gate_wait_ownership(status: u32) -> Option<bool> {
    match status {
        0 => Some(true),     // WAIT_OBJECT_0
        0x80 => Some(false), // WAIT_ABANDONED
        _ => None,
    }
}

#[cfg(any(target_os = "windows", test))]
fn relative_components(path: &[u16], mount: &[u16]) -> Option<Vec<Vec<u16>>> {
    if path.contains(&0) || mount.contains(&0) || mount.last() != Some(&(b'\\' as u16)) {
        return None;
    }
    let relative = path.strip_prefix(mount)?;
    let components = relative
        .split(|unit| *unit == b'\\' as u16)
        .map(<[u16]>::to_vec)
        .collect::<Vec<_>>();
    (!components.is_empty()
        && components.iter().all(|component| {
            !component.is_empty()
                && component != &['.' as u16]
                && component != &['.' as u16, '.' as u16]
                && !component
                    .iter()
                    .any(|unit| matches!(*unit, 0 | 47 | 58 | 92))
        }))
    .then_some(components)
}

#[cfg(any(target_os = "windows", test))]
fn is_exact_leaf_absence(status: i32) -> bool {
    // STATUS_OBJECT_NAME_NOT_FOUND only. Missing parent, denied access, a sharing
    // conflict, a reparse failure and successful open are not fresh-leaf absence.
    status == 0xc000_0034_u32 as i32
}

/// Candidate 04 D03 preimage; structural bytes only. Windows callers must also
/// obtain the SID from a trusted token and pass native SID validation.
pub(crate) fn candidate04_owner_sid_digest(sid: &[u8]) -> Option<[u8; 32]> {
    use sha2::{Digest, Sha256};
    if !(8..=68).contains(&sid.len())
        || sid[0] != 1
        || sid[1] > 15
        || sid.len() != 8 + usize::from(sid[1]) * 4
    {
        return None;
    }
    let mut digest = Sha256::new();
    digest.update(b"DisplayDeck.OwnerSidDigest.V1\0");
    digest.update((sid.len() as u32).to_le_bytes());
    digest.update(sid);
    Some(digest.finalize().into())
}

// Conservative install-only ACL admission, not a general Windows access evaluator.
// Unknown/conditional/deny ACEs fail closed; no deny-order or group-membership inference.
#[cfg(any(target_os = "windows", test))]
fn install_acl_is_read_only(owner: &[u8], acl: &[u8], ancestor: bool) -> Option<()> {
    const SYSTEM: &[u8] = &[1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0];
    const ADMINS: &[u8] = &[1, 2, 0, 0, 0, 0, 0, 5, 32, 0, 0, 0, 32, 2, 0, 0];
    // ponytail: SYSTEM/Administrators owners only. Other owners, including
    // TrustedInstaller, need an exact-cell reviewed profile before admission.
    if ![SYSTEM, ADMINS].contains(&owner)
        || acl.len() < 8
        || acl[0] != 2
        || acl[1] != 0
        || acl[6..8] != [0, 0]
        || usize::from(u16::from_le_bytes(acl[2..4].try_into().ok()?)) != acl.len()
    {
        return None;
    }
    let count = u16::from_le_bytes(acl[4..6].try_into().ok()?);
    if count == 0 || count > 64 {
        return None;
    }
    let mut offset = 8_usize;
    for _ in 0..count {
        let header = acl.get(offset..offset.checked_add(8)?)?;
        let length = usize::from(u16::from_le_bytes(header[2..4].try_into().ok()?));
        let ace = acl.get(offset..offset.checked_add(length)?)?;
        let sid = ace.get(8..)?;
        if header[0] != 0
            || header[1] & !0x1f != 0
            || sid.len() < 8
            || sid[0] != 1
            || sid[1] > 15
            || sid.len() != 8 + usize::from(sid[1]) * 4
        {
            return None;
        }
        let mask = u32::from_le_bytes(header[4..8].try_into().ok()?);
        if mask & !0xf01f_01ff != 0 {
            return None;
        }
        // Inherit-only grants do not apply to this object. Each existing child is
        // independently inspected. No creation is authorized by this check.
        if header[1] & 0x08 == 0 && ![SYSTEM, ADMINS].contains(&sid) {
            // FILE_GENERIC_READ | FILE_GENERIC_EXECUTE | GENERIC_READ | GENERIC_EXECUTE.
            let allowed = 0xa012_00a9 | if ancestor { 0x06 } else { 0 };
            // Ancestors may allow creating unrelated children, never deleting/replacing
            // existing children or modifying attributes, security, ownership or streams.
            if mask & !allowed != 0 {
                return None;
            }
        }
        offset = offset.checked_add(length)?;
    }
    // ACL capacity may include unused bytes, but only zero-filled padding is admitted.
    acl.get(offset..)?
        .iter()
        .all(|byte| *byte == 0)
        .then_some(())
}

#[cfg(target_os = "windows")]
mod platform {
    use std::{mem::size_of, ptr};

    use sha2::{Digest, Sha256};
    use windows::{
        core::{GUID, PCWSTR, PWSTR},
        Wdk::{
            Foundation::OBJECT_ATTRIBUTES,
            Storage::FileSystem::{
                NtCreateFile, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
                FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
            },
        },
        Win32::{
            Foundation::{
                CloseHandle, HANDLE, HLOCAL, NTSTATUS, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE,
                STATUS_INVALID_PARAMETER, UNICODE_STRING,
            },
            Security::Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
            Security::{
                AclSizeInformation, CreateWellKnownSid, EqualSid, GetAce, GetAclInformation,
                GetLengthSid, GetSecurityDescriptorControl, GetTokenInformation, IsValidSid,
                TokenUser, WinBuiltinAdministratorsSid, WinLocalSystemSid, ACCESS_ALLOWED_ACE,
                ACE_HEADER, ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION,
                OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SE_DACL_PRESENT,
                SE_DACL_PROTECTED, SE_SELF_RELATIVE, TOKEN_QUERY, TOKEN_USER,
            },
            Storage::FileSystem::{
                CreateFileW, FileAttributeTagInfo, FileIdInfo, FileStandardInfo, FileStreamInfo,
                GetDriveTypeW, GetFileInformationByHandleEx, GetFinalPathNameByHandleW,
                GetVolumeInformationByHandleW, GetVolumeNameForVolumeMountPointW,
                GetVolumePathNameW, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_COMPRESSED,
                FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_HIDDEN,
                FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_NOT_CONTENT_INDEXED, FILE_ATTRIBUTE_OFFLINE,
                FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_SYSTEM,
                FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
                FILE_ID_INFO, FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_READ_EA,
                FILE_SHARE_DELETE, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE,
                FILE_STANDARD_INFO, FILE_STREAM_INFO, FILE_TRAVERSE, FILE_WRITE_DATA,
                OPEN_EXISTING, READ_CONTROL, SYNCHRONIZE, VOLUME_NAME_GUID,
            },
            System::{
                Com::CoTaskMemFree,
                SystemServices::{
                    FILE_NAMED_STREAMS, FILE_PERSISTENT_ACLS, FILE_SUPPORTS_HARD_LINKS,
                },
                Threading::{GetCurrentProcess, OpenProcessToken, QueryFullProcessImageNameW},
                IO::IO_STATUS_BLOCK,
            },
            UI::Shell::{
                FOLDERID_ProgramData, FOLDERID_ProgramFiles, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
            },
        },
    };

    use super::{relative_components, D07Anchor, D07StorageFailure, D07StorageVerdict};

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
        let Some(token_sid) = token_sid() else {
            return no_go(D07StorageFailure::RuntimeSidUnproven);
        };
        let directory_anchor = match MachineDirectoryAnchor::open(token_sid) {
            Ok(anchor) => anchor,
            Err(reason) => return no_go(reason),
        };
        let Some(provision) = open_relative_file(directory_anchor.directory.0, PROVISION, false)
        else {
            return no_go(D07StorageFailure::ProvisionRecordMissing);
        };
        if !dacl_matches(provision.0, &directory_anchor.token_sid, ADMIN_READ) {
            return no_go(D07StorageFailure::ProvisionRecordDaclUnproven);
        };
        let Some(provision_evidence) = verify_identity(
            provision.0,
            ObjectKind::File {
                expected_length: PROVISION_LENGTH,
            },
        ) else {
            return no_go(D07StorageFailure::ProvisionRecordIdentityUnproven);
        };
        let Some(actor) = open_relative_file(directory_anchor.directory.0, ACTOR, true) else {
            return no_go(D07StorageFailure::ActorRecordMissing);
        };
        if !dacl_matches(actor.0, &directory_anchor.token_sid, RECORD_SLOT_WRITE) {
            return no_go(D07StorageFailure::ActorRecordDaclUnproven);
        }
        let Some(actor_evidence) = verify_identity(
            actor.0,
            ObjectKind::File {
                expected_length: ACTOR_LENGTH,
            },
        ) else {
            return no_go(D07StorageFailure::ActorRecordIdentityUnproven);
        };
        if !directory_anchor.records_match(&provision_evidence, &actor_evidence) {
            return no_go(D07StorageFailure::AnchorChainMismatch);
        }
        D07StorageVerdict::Go(D07Anchor {
            directory_anchor,
            provision,
            actor,
            provision_evidence,
            actor_evidence,
        })
    }

    pub(super) struct MachineDirectoryAnchor {
        volume_root: HeldHandle,
        program_data_handles: Vec<HeldHandle>,
        directory: HeldHandle,
        pub(super) token_sid: TokenSid,
        components: Vec<Vec<u16>>,
        volume_root_evidence: ObjectEvidence,
        program_data_evidence: Vec<ObjectEvidence>,
        directory_evidence: ObjectEvidence,
        volume: VolumeProfile,
    }

    impl MachineDirectoryAnchor {
        fn open(token_sid: TokenSid) -> Result<Self, D07StorageFailure> {
            let Ok(program_data) = known_folder(&FOLDERID_ProgramData) else {
                return Err(D07StorageFailure::ProgramDataUnavailable);
            };
            let Some((volume_name, components)) = known_folder_location(&program_data) else {
                return Err(D07StorageFailure::VolumePathUnproven);
            };
            if components.len() > 16 {
                return Err(D07StorageFailure::VolumePathUnproven);
            }
            let Some(volume_root) = open_volume_root(&volume_name) else {
                return Err(D07StorageFailure::VolumeRootOpenUnproven);
            };
            let Some(volume_root_evidence) = verify_root(volume_root.0) else {
                return Err(D07StorageFailure::VolumeRootIdentityUnproven);
            };
            let Some(volume) = volume_profile(volume_root.0) else {
                return Err(D07StorageFailure::LocalFixedNtfsUnproven);
            };
            if volume_root_evidence.id.VolumeSerialNumber != u64::from(volume.serial)
                || volume_root_evidence.final_path != volume.root_path
                || volume.guid_root != volume.root_path
            {
                return Err(D07StorageFailure::LocalFixedNtfsUnproven);
            }
            let mut parent = volume_root.0;
            let mut program_data_handles = Vec::with_capacity(components.len());
            let mut program_data_evidence = Vec::with_capacity(components.len());
            for component in &components {
                let Some(handle) = open_relative_directory_units(parent, component) else {
                    return Err(D07StorageFailure::ProgramDataComponentMissing);
                };
                let Some(evidence) = verify_identity(handle.0, ObjectKind::Directory) else {
                    return Err(D07StorageFailure::ProgramDataComponentIdentityUnproven);
                };
                parent = handle.0;
                program_data_handles.push(handle);
                program_data_evidence.push(evidence);
            }
            let Some(directory) = open_relative_directory(parent, DIRECTORY) else {
                return Err(D07StorageFailure::DisplayDeckDirectoryMissing);
            };
            if !dacl_matches(directory.0, &token_sid, DIRECTORY_TRAVERSE) {
                return Err(D07StorageFailure::DisplayDeckDirectoryDaclUnproven);
            }
            let Some(directory_evidence) = verify_identity(directory.0, ObjectKind::Directory)
            else {
                return Err(D07StorageFailure::DisplayDeckDirectoryIdentityUnproven);
            };
            let anchor = Self {
                volume_root,
                program_data_handles,
                directory,
                token_sid,
                components,
                volume_root_evidence,
                program_data_evidence,
                directory_evidence,
                volume,
            };
            anchor
                .revalidate()
                .then_some(anchor)
                .ok_or(D07StorageFailure::AnchorChainMismatch)
        }

        pub(super) fn revalidate(&self) -> bool {
            volume_profile(self.volume_root.0).as_ref() == Some(&self.volume)
                && verify_root(self.volume_root.0).as_ref() == Some(&self.volume_root_evidence)
                && self.program_data_handles.len() == self.program_data_evidence.len()
                && self
                    .program_data_handles
                    .iter()
                    .zip(&self.program_data_evidence)
                    .all(|(handle, expected)| {
                        verify_identity(handle.0, ObjectKind::Directory).as_ref() == Some(expected)
                    })
                && verify_directory(self.directory.0, &self.token_sid).as_ref()
                    == Some(&self.directory_evidence)
                && directory_chain_matches(
                    &self.volume_root_evidence,
                    &self.components,
                    &self.program_data_evidence,
                    &self.directory_evidence,
                )
        }

        pub(super) fn records_match(
            &self,
            provision: &ObjectEvidence,
            actor: &ObjectEvidence,
        ) -> bool {
            anchor_chain_matches(
                &self.volume_root_evidence,
                &self.components,
                &self.program_data_evidence,
                &self.directory_evidence,
                provision,
                actor,
            )
        }
    }

    // Thread-owned mutex guard: never move ownership to a different thread.
    pub(crate) struct ProvisionMachineGate {
        handle: HeldHandle,
        _thread: std::marker::PhantomData<std::rc::Rc<()>>,
    }

    impl ProvisionMachineGate {
        pub(crate) fn acquire(owner_token: HANDLE, expected_owner: [u8; 32]) -> Option<Self> {
            use windows::{
                core::{w, PWSTR},
                Win32::{
                    Foundation::LocalFree,
                    Security::{
                        Authorization::{
                            ConvertSidToStringSidW,
                            ConvertStringSecurityDescriptorToSecurityDescriptorW, SE_KERNEL_OBJECT,
                        },
                        SECURITY_ATTRIBUTES,
                    },
                    System::Threading::{CreateMutexExW, WaitForSingleObject},
                },
            };
            if !current_process_is_local_system()
                || expected_owner == [0; 32]
                || token_sid_digest(owner_token) != Some(expected_owner)
            {
                return None;
            }
            let runtime = token_sid_for_handle(owner_token)?;
            let mut sid_text = PWSTR::null();
            // SAFETY: runtime holds a validated SID; the returned string is LocalAlloc-owned.
            unsafe { ConvertSidToStringSidW(runtime.as_psid(), &mut sid_text) }.ok()?;
            // SAFETY: successful conversion returns a terminated UTF-16 SID string.
            let sid = unsafe { sid_text.to_string() };
            // SAFETY: conversion allocated this string; the Rust copy no longer borrows it.
            unsafe { LocalFree(Some(HLOCAL(sid_text.0.cast()))) };
            let sid = sid.ok()?;
            // Source candidate: SYSTEM full, Administrators read/synchronize,
            // designated runtime read/synchronize/modify-state. Never default DACL.
            let sddl = format!(
                "O:SYG:SYD:P(A;;0x001f0001;;;SY)(A;;0x00120000;;;BA)(A;;0x00120001;;;{sid})"
            )
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
            let mut descriptor = PSECURITY_DESCRIPTOR::default();
            // SAFETY: terminated local SDDL and valid output pointer; freed after creation.
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    PCWSTR(sddl.as_ptr()),
                    1,
                    &mut descriptor,
                    None,
                )
            }
            .ok()?;
            let attributes = SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor.0,
                bInheritHandle: false.into(),
            };
            // SAFETY: descriptor remains live for this call; no initial ownership is requested.
            let opened = unsafe {
                CreateMutexExW(
                    Some(&attributes),
                    w!("Global\\DisplayDeck.MaintenanceMutation.v1"),
                    0,
                    0x00120001,
                )
            };
            // SAFETY: conversion returned LocalAlloc-owned descriptor, no longer borrowed.
            unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
            let handle = HeldHandle(opened.ok()?);
            // CreateMutexEx ignores supplied security when the object already exists.
            if !exact_object_security(
                handle.0,
                &runtime,
                SE_KERNEL_OBJECT,
                [0x001f0001, 0x00120000, 0x00120001],
            ) {
                return None;
            }
            // SAFETY: live mutex handle with SYNCHRONIZE; zero timeout never waits.
            let wait = unsafe { WaitForSingleObject(handle.0, 0) };
            let clean_acquisition = super::gate_wait_ownership(wait.0)?;
            let guard = Self {
                handle,
                _thread: std::marker::PhantomData,
            };
            // Abandoned grants ownership but is NOT permission to proceed.
            // Drop releases that ownership. Recovery inspection is future work.
            if !clean_acquisition
                || !exact_object_security(
                    guard.handle.0,
                    &runtime,
                    SE_KERNEL_OBJECT,
                    [0x001f0001, 0x00120000, 0x00120001],
                )
            {
                return None;
            }
            Some(guard)
        }
    }

    impl Drop for ProvisionMachineGate {
        fn drop(&mut self) {
            // SAFETY: the !Send guard owns this mutex on the acquiring thread.
            // A failure leaves ownership until thread exit; it never authorizes work.
            let _ = unsafe { windows::Win32::System::Threading::ReleaseMutex(self.handle.0) };
        }
    }

    // A retained read-only observation, NOT a fresh-create/write grant. Absence can
    // race with non-cooperating writers: the gate does not replace exclusive CREATE_NEW.
    pub(crate) struct FreshProvisionObservation {
        directory: MachineDirectoryAnchor,
    }

    impl FreshProvisionObservation {
        pub(crate) fn observe(
            _gate: &ProvisionMachineGate,
            owner_token: HANDLE,
            expected_owner: [u8; 32],
        ) -> Option<Self> {
            if !current_process_is_local_system()
                || expected_owner == [0; 32]
                || token_sid_digest(owner_token) != Some(expected_owner)
            {
                return None;
            }
            let directory =
                MachineDirectoryAnchor::open(token_sid_for_handle(owner_token)?).ok()?;
            let observation = Self { directory };
            observation.reobserve().then_some(observation)
        }

        pub(crate) fn reobserve(&self) -> bool {
            current_process_is_local_system()
                && self.directory.revalidate()
                && [PROVISION, ACTOR].into_iter().all(|name| {
                    let mut units = name.encode_utf16().collect::<Vec<_>>();
                    units.push(0);
                    match open_existing(
                        Some(self.directory.directory.0),
                        &units,
                        false,
                        false,
                        Default::default(),
                    ) {
                        // Any existing object blocks fresh creation, including zero-length,
                        // old terminal or corrupt files. Drop closes without reading/mutating.
                        Ok(_existing) => false,
                        Err(status) => super::is_exact_leaf_absence(status.0),
                    }
                })
                && self.directory.revalidate()
        }
    }

    fn no_go(reason: D07StorageFailure) -> D07StorageVerdict {
        D07StorageVerdict::NoGo(reason)
    }

    pub(crate) struct ProtectedInstall {
        parents: Vec<(HeldHandle, ObjectEvidence, [u8; 32])>,
        volume: VolumeProfile,
        actor_path: Vec<u16>,
    }

    #[derive(PartialEq)]
    pub(crate) struct InstallFileEvidence {
        identity: ObjectEvidence,
        security: [u8; 32],
    }

    impl ProtectedInstall {
        pub(crate) fn open() -> Option<Self> {
            let program_files = known_folder(&FOLDERID_ProgramFiles).ok()?;
            let (volume_name, mut components) = known_folder_location(&program_files)?;
            if components.len() > 16 {
                return None;
            }
            components.push(DIRECTORY.encode_utf16().collect());
            let root = open_volume_root(&volume_name)?;
            let identity = verify_root(root.0)?;
            let volume = volume_profile(root.0)?;
            if identity.id.VolumeSerialNumber != u64::from(volume.serial)
                || identity.final_path != volume.root_path
                || volume.guid_root != volume.root_path
            {
                return None;
            }
            let security = install_security(root.0, true)?;
            let mut parents = vec![(root, identity, security)];
            for (index, component) in components.iter().enumerate() {
                let parent = parents.last()?;
                let child = open_relative_directory_units(parent.0 .0, component)?;
                let identity = verify_identity(child.0, ObjectKind::Directory)?;
                if identity.id.VolumeSerialNumber != u64::from(volume.serial)
                    || !is_direct_child_units(&parent.1.final_path, &identity.final_path, component)
                {
                    return None;
                }
                let security = install_security(child.0, index + 1 != components.len())?;
                parents.push((child, identity, security));
            }
            let mut actor_path = program_files.strip_suffix(&[0])?.to_vec();
            actor_path.extend("\\DisplayDeck\\".encode_utf16());
            actor_path.extend(crate::provision_service::ACTOR_IMAGE_NAME.encode_utf16());
            let install = Self {
                parents,
                volume,
                actor_path,
            };
            install.revalidate().then_some(install)
        }

        pub(crate) fn actor_path(&self) -> &[u16] {
            &self.actor_path
        }

        pub(crate) fn revalidate(&self) -> bool {
            let Some(root) = self.parents.first() else {
                return false;
            };
            volume_profile(root.0 .0).as_ref() == Some(&self.volume)
                && self
                    .parents
                    .iter()
                    .enumerate()
                    .all(|(index, (handle, identity, security))| {
                        let kind = if index == 0 {
                            ObjectKind::Root
                        } else {
                            ObjectKind::Directory
                        };
                        verify_identity(handle.0, kind).as_ref() == Some(identity)
                            && install_security(handle.0, index + 1 != self.parents.len()).as_ref()
                                == Some(security)
                    })
        }

        // This is a process NAME observation only, not loaded-image byte identity.
        // Pre-open image replacement/launch provenance still blocks a provision grant.
        pub(crate) fn process_image_name_matches(&self) -> bool {
            let mut path = [0_u16; 32_768];
            let mut length = path.len() as u32;
            // SAFETY: current-process pseudo-handle and bounded writable UTF-16 buffer.
            if unsafe {
                QueryFullProcessImageNameW(
                    GetCurrentProcess(),
                    Default::default(),
                    PWSTR(path.as_mut_ptr()),
                    &mut length,
                )
            }
            .is_err()
            {
                return false;
            }
            let length = length as usize;
            length != 0
                && length < path.len()
                && path[length] == 0
                && normalize_final_path(&path[..length])
                    .zip(normalize_final_path(&self.actor_path))
                    .is_some_and(|(actual, expected)| actual == expected)
        }

        pub(crate) fn open_file(&self, name: &str) -> Option<std::fs::File> {
            use std::os::windows::io::FromRawHandle;
            if ![
                crate::provision_service::ACTOR_IMAGE_NAME,
                crate::provision_service::MANIFEST_NAME,
                crate::provision_service::MANIFEST_SIGNATURE_NAME,
            ]
            .contains(&name)
            {
                return None;
            }
            let mut units = name.encode_utf16().collect::<Vec<_>>();
            units.push(0);
            let handle = open(
                Some(self.parents.last()?.0 .0),
                &units,
                false,
                false,
                FILE_SHARE_READ,
            )?;
            let handle = std::mem::ManuallyDrop::new(handle);
            // SAFETY: transfer the single owned NT handle to File; HeldHandle will not close it.
            Some(unsafe { std::fs::File::from_raw_handle(handle.0 .0) })
        }

        pub(crate) fn file_evidence(
            &self,
            file: &std::fs::File,
            name: &str,
            length: u64,
        ) -> Option<InstallFileEvidence> {
            use std::os::windows::io::AsRawHandle;
            let handle = HANDLE(file.as_raw_handle());
            let identity = verify_identity(
                handle,
                ObjectKind::File {
                    expected_length: i64::try_from(length).ok()?,
                },
            )?;
            let parent = self.parents.last()?;
            if identity.id.VolumeSerialNumber != u64::from(self.volume.serial)
                || !is_direct_child(&parent.1.final_path, &identity.final_path, name)
            {
                return None;
            }
            Some(InstallFileEvidence {
                identity,
                security: install_security(handle, false)?,
            })
        }
    }

    fn install_security(handle: HANDLE, ancestor: bool) -> Option<[u8; 32]> {
        let mut owner = PSID::default();
        let mut dacl = ptr::null_mut();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: all outputs are initialized and remain owned until LocalFree below.
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
        let evidence = (|| {
            if result.0 != 0 || owner.0.is_null() || dacl.is_null() || descriptor.0.is_null() {
                return None;
            }
            let mut control = 0_u16;
            let mut revision = 0_u32;
            let mut info = ACL_SIZE_INFORMATION::default();
            // SAFETY: these pointers are native-owned parts of the successful descriptor.
            unsafe {
                GetSecurityDescriptorControl(descriptor, &mut control, &mut revision).ok()?;
                GetAclInformation(
                    dacl,
                    (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
                    size_of::<ACL_SIZE_INFORMATION>() as u32,
                    AclSizeInformation,
                )
                .ok()?;
                if !IsValidSid(owner).as_bool() {
                    return None;
                }
            }
            if revision != 1 || control & SE_DACL_PRESENT.0 == 0 {
                return None;
            }
            // SAFETY: IsValidSid succeeded for the live descriptor's owner.
            let sid_length = unsafe { GetLengthSid(owner) } as usize;
            let acl_length =
                usize::try_from(info.AclBytesInUse.checked_add(info.AclBytesFree)?).ok()?;
            if !(8..=68).contains(&sid_length) || !(8..=65_535).contains(&acl_length) {
                return None;
            }
            // SAFETY: native APIs validated both descriptor components and reported their sizes.
            let (owner, acl) = unsafe {
                (
                    std::slice::from_raw_parts(owner.0.cast::<u8>(), sid_length),
                    std::slice::from_raw_parts(dacl.cast::<u8>(), acl_length),
                )
            };
            super::install_acl_is_read_only(owner, acl, ancestor)?;
            let mut digest = Sha256::new();
            digest.update(control.to_le_bytes());
            digest.update(owner);
            digest.update(acl);
            Some(digest.finalize().into())
        })();
        if !descriptor.0.is_null() {
            // SAFETY: GetSecurityInfo uses LocalAlloc, including a partial returned descriptor.
            unsafe { windows::Win32::Foundation::LocalFree(Some(HLOCAL(descriptor.0))) };
        }
        evidence
    }

    fn known_folder(folder: &GUID) -> Result<Vec<u16>, ()> {
        // SAFETY: the shell returns a CoTaskMem-allocated, NUL-terminated UTF-16 string.
        let raw = unsafe { SHGetKnownFolderPath(folder, KF_FLAG_DEFAULT, None) }.map_err(|_| ())?;
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

    fn known_folder_location(program_data: &[u16]) -> Option<(Vec<u16>, Vec<Vec<u16>>)> {
        let path = program_data.strip_suffix(&[0])?;
        let mut mount_buffer = [0_u16; 32_768];
        // SAFETY: `program_data` is a NUL-terminated known-folder path and output is bounded.
        unsafe { GetVolumePathNameW(PCWSTR(program_data.as_ptr()), &mut mount_buffer) }.ok()?;
        let mount = terminated_value(&mount_buffer)?;
        let components = relative_components(path, mount)?;

        let mut volume_buffer = [0_u16; 64];
        // SAFETY: the returned mount path is NUL-terminated and output is bounded.
        unsafe {
            GetVolumeNameForVolumeMountPointW(PCWSTR(mount_buffer.as_ptr()), &mut volume_buffer)
        }
        .ok()?;
        let normalized = normalize_final_path(terminated_value(&volume_buffer)?)?;
        (volume_guid_root(&normalized).as_deref() == Some(normalized.as_slice())).then_some(())?;
        let mut volume_name = normalized;
        volume_name.push(0);
        Some((volume_name, components))
    }

    fn terminated_value(buffer: &[u16]) -> Option<&[u16]> {
        let end = buffer.iter().position(|unit| *unit == 0)?;
        (end != 0 && buffer[end..].iter().all(|unit| *unit == 0)).then_some(&buffer[..end])
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

    fn open_volume_root(volume_name: &[u16]) -> Option<HeldHandle> {
        let access =
            (FILE_TRAVERSE | FILE_READ_EA | FILE_READ_ATTRIBUTES | READ_CONTROL | SYNCHRONIZE).0;
        // SAFETY: this is a validated NUL-terminated volume GUID root with no output pointers.
        let handle = unsafe {
            CreateFileW(
                PCWSTR(volume_name.as_ptr()),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                None,
            )
        }
        .ok()?;
        Some(HeldHandle(handle))
    }
    fn open_relative_directory(root: HANDLE, name: &str) -> Option<HeldHandle> {
        open_relative_directory_units(root, &name.encode_utf16().collect::<Vec<_>>())
    }
    fn open_relative_directory_units(root: HANDLE, name: &[u16]) -> Option<HeldHandle> {
        let mut name = name.to_vec();
        name.push(0);
        open(Some(root), &name, true, false, FILE_SHARE_READ)
    }
    fn open_relative_file(root: HANDLE, name: &str, writable: bool) -> Option<HeldHandle> {
        let mut name = name.encode_utf16().collect::<Vec<_>>();
        name.push(0);
        open(Some(root), &name, false, writable, Default::default())
    }
    fn open(
        root: Option<HANDLE>,
        name: &[u16],
        directory: bool,
        writable: bool,
        share: FILE_SHARE_MODE,
    ) -> Option<HeldHandle> {
        open_existing(root, name, directory, writable, share).ok()
    }

    fn open_existing(
        root: Option<HANDLE>,
        name: &[u16],
        directory: bool,
        writable: bool,
        share: FILE_SHARE_MODE,
    ) -> Result<HeldHandle, NTSTATUS> {
        let units = name.strip_suffix(&[0]).ok_or(STATUS_INVALID_PARAMETER)?;
        if units.is_empty() || units.contains(&0) {
            return Err(STATUS_INVALID_PARAMETER);
        }
        let bytes = units.len().checked_mul(2).ok_or(STATUS_INVALID_PARAMETER)?;
        let length = u16::try_from(bytes).map_err(|_| STATUS_INVALID_PARAMETER)?;
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
                share,
                FILE_OPEN,
                options,
                None,
                0,
            )
        };
        if status.is_ok() {
            Ok(HeldHandle(handle))
        } else {
            Err(status)
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub(super) struct ObjectEvidence {
        id: FILE_ID_INFO,
        attributes: u32,
        stream_name: Vec<u16>,
        final_path: Vec<u16>,
    }

    #[derive(Clone, Copy)]
    pub(super) enum ObjectKind {
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

    pub(super) fn verify_identity(handle: HANDLE, kind: ObjectKind) -> Option<ObjectEvidence> {
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
        is_direct_child_units(parent, child, &name.encode_utf16().collect::<Vec<_>>())
    }

    fn is_direct_child_units(parent: &[u16], child: &[u16], name: &[u16]) -> bool {
        let mut expected = parent.to_vec();
        if expected.last() != Some(&(b'\\' as u16)) {
            expected.push(b'\\' as u16);
        }
        let Ok(name) = String::from_utf16(name) else {
            return false;
        };
        expected.extend(name.to_lowercase().encode_utf16());
        child == expected
    }

    pub(super) fn anchor_chain_matches(
        volume_root: &ObjectEvidence,
        components: &[Vec<u16>],
        program_data: &[ObjectEvidence],
        directory: &ObjectEvidence,
        provision: &ObjectEvidence,
        actor: &ObjectEvidence,
    ) -> bool {
        directory_chain_matches(volume_root, components, program_data, directory)
            && [provision, actor]
                .iter()
                .all(|evidence| evidence.id.VolumeSerialNumber == volume_root.id.VolumeSerialNumber)
            && is_direct_child(&directory.final_path, &provision.final_path, PROVISION)
            && is_direct_child(&directory.final_path, &actor.final_path, ACTOR)
    }

    fn directory_chain_matches(
        volume_root: &ObjectEvidence,
        components: &[Vec<u16>],
        program_data: &[ObjectEvidence],
        directory: &ObjectEvidence,
    ) -> bool {
        if components.len() != program_data.len() || components.is_empty() {
            return false;
        }
        let serial = volume_root.id.VolumeSerialNumber;
        let mut parent = volume_root;
        for (component, evidence) in components.iter().zip(program_data) {
            if evidence.id.VolumeSerialNumber != serial
                || !is_direct_child_units(&parent.final_path, &evidence.final_path, component)
            {
                return false;
            }
            parent = evidence;
        }
        directory.id.VolumeSerialNumber == serial
            && is_direct_child(&parent.final_path, &directory.final_path, DIRECTORY)
    }

    fn dacl_matches(handle: HANDLE, runtime: &TokenSid, runtime_mask: u32) -> bool {
        exact_object_security(
            handle,
            runtime,
            SE_FILE_OBJECT,
            [SYSTEM_FULL, ADMIN_READ, runtime_mask],
        )
    }

    fn exact_object_security(
        handle: HANDLE,
        runtime: &TokenSid,
        kind: windows::Win32::Security::Authorization::SE_OBJECT_TYPE,
        masks: [u32; 3],
    ) -> bool {
        let mut owner = PSID::default();
        let mut dacl = ptr::null_mut();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: descriptor is released exactly once below.
        let result = unsafe {
            GetSecurityInfo(
                handle,
                kind,
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
                (system.as_psid(), masks[0]),
                (admins.as_psid(), masks[1]),
                (runtime.as_psid(), masks[2]),
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
        let result = token_sid_for_handle(token);
        unsafe { CloseHandle(token) }.ok()?;
        result
    }
    fn token_sid_for_handle(token: HANDLE) -> Option<TokenSid> {
        (|| {
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
        })()
    }
    pub(super) fn current_process_is_local_system() -> bool {
        token_sid()
            .zip(well_known_sid(WinLocalSystemSid))
            .is_some_and(|(actual, expected)| unsafe {
                EqualSid(actual.as_psid(), expected.as_psid()).is_ok()
            })
    }
    pub(super) fn token_sid_digest(token: HANDLE) -> Option<[u8; 32]> {
        let sid = token_sid_for_handle(token)?;
        let length = unsafe { windows::Win32::Security::GetLengthSid(sid.as_psid()) } as usize;
        let system = well_known_sid(WinLocalSystemSid)?;
        if !(8..=sid.storage.0.len()).contains(&length)
            || unsafe { EqualSid(sid.as_psid(), system.as_psid()) }.is_ok()
        {
            return None;
        }
        super::candidate04_owner_sid_digest(&sid.storage.0[..length])
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
pub(crate) use platform::{
    FreshProvisionObservation, InstallFileEvidence, ProtectedInstall, ProvisionMachineGate,
};

#[cfg(target_os = "windows")]
pub(crate) fn current_process_is_local_system() -> bool {
    platform::current_process_is_local_system()
}

#[cfg(target_os = "windows")]
pub(crate) fn token_sid_digest(token: windows::Win32::Foundation::HANDLE) -> Option<[u8; 32]> {
    platform::token_sid_digest(token)
}

#[cfg(target_os = "windows")]
pub struct D07Anchor {
    directory_anchor: platform::MachineDirectoryAnchor,
    provision: platform::HeldHandle,
    actor: platform::HeldHandle,
    provision_evidence: platform::ObjectEvidence,
    actor_evidence: platform::ObjectEvidence,
}
#[cfg(target_os = "windows")]
impl D07Anchor {
    /// The retained handle closes the post-D07 replacement race; write code must revalidate it first.
    pub fn revalidate_before_actor_write(&self) -> bool {
        self.directory_anchor.revalidate()
            && platform::verify_file(
                self.provision.0,
                &self.directory_anchor.token_sid,
                0x0012_0089,
                12_288,
            )
            .as_ref()
                == Some(&self.provision_evidence)
            && platform::verify_file(
                self.actor.0,
                &self.directory_anchor.token_sid,
                0x0012_008b,
                135_168,
            )
            .as_ref()
                == Some(&self.actor_evidence)
            && self
                .directory_anchor
                .records_match(&self.provision_evidence, &self.actor_evidence)
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
    VolumePathUnproven,
    VolumeRootOpenUnproven,
    VolumeRootIdentityUnproven,
    LocalFixedNtfsUnproven,
    RuntimeSidUnproven,
    ProgramDataComponentMissing,
    ProgramDataComponentIdentityUnproven,
    DisplayDeckDirectoryMissing,
    DisplayDeckDirectoryDaclUnproven,
    DisplayDeckDirectoryIdentityUnproven,
    ProvisionRecordMissing,
    ProvisionRecordDaclUnproven,
    ProvisionRecordIdentityUnproven,
    ActorRecordMissing,
    ActorRecordDaclUnproven,
    ActorRecordIdentityUnproven,
    AnchorChainMismatch,
    RevalidationFailed,
}
impl D07StorageFailure {
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotWindows => "D07_NOT_WINDOWS",
            Self::ProgramDataUnavailable => "D07_PROGRAM_DATA_UNAVAILABLE",
            Self::VolumePathUnproven => "D07_VOLUME_PATH_UNPROVEN",
            Self::VolumeRootOpenUnproven => "D07_VOLUME_ROOT_OPEN_UNPROVEN",
            Self::VolumeRootIdentityUnproven => "D07_VOLUME_ROOT_IDENTITY_UNPROVEN",
            Self::LocalFixedNtfsUnproven => "D07_LOCAL_FIXED_NTFS_UNPROVEN",
            Self::RuntimeSidUnproven => "D07_RUNTIME_SID_UNPROVEN",
            Self::ProgramDataComponentMissing => "D07_PROGRAM_DATA_COMPONENT_MISSING",
            Self::ProgramDataComponentIdentityUnproven => {
                "D07_PROGRAM_DATA_COMPONENT_IDENTITY_UNPROVEN"
            }
            Self::DisplayDeckDirectoryMissing => "D07_DISPLAYDECK_DIRECTORY_MISSING",
            Self::DisplayDeckDirectoryDaclUnproven => "D07_DISPLAYDECK_DIRECTORY_DACL_UNPROVEN",
            Self::DisplayDeckDirectoryIdentityUnproven => {
                "D07_DISPLAYDECK_DIRECTORY_IDENTITY_UNPROVEN"
            }
            Self::ProvisionRecordMissing => "D07_PROVISION_RECORD_MISSING",
            Self::ProvisionRecordDaclUnproven => "D07_PROVISION_RECORD_DACL_UNPROVEN",
            Self::ProvisionRecordIdentityUnproven => "D07_PROVISION_RECORD_IDENTITY_UNPROVEN",
            Self::ActorRecordMissing => "D07_ACTOR_RECORD_MISSING",
            Self::ActorRecordDaclUnproven => "D07_ACTOR_RECORD_DACL_UNPROVEN",
            Self::ActorRecordIdentityUnproven => "D07_ACTOR_RECORD_IDENTITY_UNPROVEN",
            Self::AnchorChainMismatch => "D07_ANCHOR_CHAIN_MISMATCH",
            Self::RevalidationFailed => "D07_REVALIDATION_FAILED",
        }
    }
}
#[cfg(target_os = "windows")]
pub fn inspect_machine_actor_storage() -> D07StorageVerdict {
    platform::inspect()
}
#[cfg(not(target_os = "windows"))]
pub fn inspect_machine_actor_storage() -> D07StorageVerdict {
    D07StorageVerdict::NoGo(D07StorageFailure::NotWindows)
}
#[cfg(test)]
mod tests {
    #[test]
    fn gate_wait_distinguishes_abandoned_ownership_from_permission() {
        assert_eq!(super::gate_wait_ownership(0), Some(true));
        // Must release ownership, but must not inspect/create as a clean acquisition.
        assert_eq!(super::gate_wait_ownership(0x80), Some(false));
        for status in [0x102, u32::MAX, 1, 0x81] {
            assert_eq!(super::gate_wait_ownership(status), None);
        }
    }

    use super::*;

    #[test]
    fn fresh_absence_never_conflates_open_failures_with_missing_leaf() {
        assert!(is_exact_leaf_absence(0xc000_0034_u32 as i32));
        for status in [
            0,
            1,
            0x0000_0103, // success / informational / pending
            0xc000_000d, // invalid parameter
            0xc000_0022, // access denied
            0xc000_0033, // invalid object name
            0xc000_0035, // name collision
            0xc000_003a, // missing path, not a proven missing leaf
            0xc000_0043, // sharing violation
            0xc000_0056, // delete pending
            0xc000_00ba, // existing directory
            0xc000_050b, // reparse encountered
            0xffff_ffff_u32,
        ] {
            assert!(!is_exact_leaf_absence(status as i32), "status {status:08x}");
        }
        #[cfg(target_os = "windows")]
        assert!(is_exact_leaf_absence(
            windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND.0
        ));
    }

    #[test]
    fn install_acl_rejects_untrusted_writes_and_malformed_entries() {
        let system = [1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0];
        let admins = [1, 2, 0, 0, 0, 0, 0, 5, 32, 0, 0, 0, 32, 2, 0, 0];
        let user = [1, 1, 0, 0, 0, 0, 0, 5, 11, 0, 0, 0];
        let acl = |mask: u32, flags: u8, kind: u8| {
            let mut bytes = vec![2, 0, 0, 0, 3, 0, 0, 0];
            for (sid, rights, ace_flags, ace_type) in [
                (system.as_slice(), 0x001f_01ff_u32, 0, 0),
                (admins.as_slice(), 0x001f_01ff_u32, 0, 0),
                (user.as_slice(), mask, flags, kind),
            ] {
                bytes.extend([ace_type, ace_flags]);
                bytes.extend(((8 + sid.len()) as u16).to_le_bytes());
                bytes.extend(rights.to_le_bytes());
                bytes.extend(sid);
            }
            let length = bytes.len() as u16;
            bytes[2..4].copy_from_slice(&length.to_le_bytes());
            bytes
        };
        let read = acl(0x0012_00a9, 0x13, 0);
        assert_eq!(install_acl_is_read_only(&system, &read, false), Some(()));
        assert_eq!(install_acl_is_read_only(&admins, &read, false), Some(()));
        assert_eq!(install_acl_is_read_only(&user, &read, false), None);
        for mask in [
            0x2, 0x4, 0x10, 0x40, 0x100, 0x10000, 0x40000, 0x80000, 0x10000000, 0x40000000,
            0x02000000, 0x200,
        ] {
            assert_eq!(
                install_acl_is_read_only(&system, &acl(mask, 0, 0), false),
                None,
                "mask {mask:x}"
            );
        }
        assert_eq!(
            install_acl_is_read_only(&system, &acl(0x6, 0, 0), true),
            Some(())
        );
        assert_eq!(
            install_acl_is_read_only(&system, &acl(0x40, 0, 0), true),
            None
        );
        assert_eq!(
            install_acl_is_read_only(&system, &acl(0xa000_0000, 0, 0), false),
            Some(())
        );
        assert_eq!(
            install_acl_is_read_only(&system, &acl(0x1000_0000, 0x0b, 0), false),
            Some(())
        );
        for kind in [1, 5, 9, 0xff] {
            assert_eq!(
                install_acl_is_read_only(&system, &acl(0x0012_00a9, 0x08, kind), false),
                None
            );
        }
        assert_eq!(
            install_acl_is_read_only(&system, &acl(0, 0x80, 0), false),
            None
        );
        for length in 0..read.len() {
            assert_eq!(
                install_acl_is_read_only(&system, &read[..length], false),
                None
            );
        }
        for (offset, value) in [
            (0, 4),
            (1, 1),
            (4, 65),
            (6, 1),
            (10, 0),
            (15, 2),
            (16, 2),
            (17, 16),
        ] {
            let mut malformed = read.clone();
            malformed[offset] = value;
            assert_eq!(install_acl_is_read_only(&system, &malformed, false), None);
        }
    }

    #[test]
    fn non_windows_storage_is_always_no_go() {
        #[cfg(not(target_os = "windows"))]
        assert!(matches!(
            inspect_machine_actor_storage(),
            D07StorageVerdict::NoGo(D07StorageFailure::NotWindows)
        ));
    }

    #[test]
    fn d07_relative_path_and_failure_codes_are_bounded() {
        let path = r"C:\ProgramData\DisplayDeck"
            .encode_utf16()
            .collect::<Vec<_>>();
        let mount = r"C:\".encode_utf16().collect::<Vec<_>>();
        assert_eq!(
            relative_components(&path, &mount),
            Some(vec![
                "ProgramData".encode_utf16().collect(),
                "DisplayDeck".encode_utf16().collect(),
            ])
        );
        assert!(relative_components(
            &r"C:\ProgramData\..\Windows"
                .encode_utf16()
                .collect::<Vec<_>>(),
            &mount,
        )
        .is_none());
        assert_eq!(
            D07StorageFailure::AnchorChainMismatch.code(),
            "D07_ANCHOR_CHAIN_MISMATCH"
        );
    }
}
