use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(any(target_os = "windows", test))]
const SERVICE_ARGUMENT: &str = "--provision-service";

#[cfg(target_os = "windows")]
pub(crate) const ACTOR_IMAGE_NAME: &str = "displaydeck-actor.exe";
pub(crate) const MANIFEST_NAME: &str = "DisplayDeckProvisionManifestV1.json";
pub(crate) const MANIFEST_SIGNATURE_NAME: &str = "DisplayDeckProvisionManifestV1.p7s";
const MANIFEST_SCHEMA: &str = "DisplayDeckProvisionManifestV1";
const WIRE_PROFILE_ID: &str = "DD-FR-002-WIRE-PROFILE-V1-CANDIDATE-04";
const WIRE_SEMANTIC_MANIFEST_SHA256: &str =
    "4211b04dc0f456f3ca9d8e3f527bb27f31bfa96dc7e3e26ab62ca69534375210";
const WIRE_ARTIFACT_INDEX_SHA256: &str =
    "de757449851e60280e949f5000072fc4edde655a1053ef587a46b30a2e246b9a";
const PROVISION_RECORD_LAYOUT_SHA256: &str =
    "3aa61ac1fc0fd8ddaf43803d4eb482f05110aed70fc94bcc5c3231f633438433";
const MACHINE_DATA_DIRECTORY: &str = "FOLDERID_ProgramData/DisplayDeck";
const PROVISION_RECORD_NAME: &str = "MachineActorProvisionRecordV1";
const MACHINE_ACTOR_RECORD_NAME: &str = "MachineActorRecordV1";
const MAX_MANIFEST_BYTES: usize = 4 * 1024;
const MAX_SIGNATURE_BYTES: usize = 64 * 1024;
const MAX_ACTOR_BYTES: u64 = 64 * 1024 * 1024;

// Unconfigured sentinels: no candidate may authorize provisioning from these values.
// The final manifest pin cannot be embedded into the actor whose full-file hash it contains;
// its eventual external trust binding must be resolved before candidate activation.
const EXPECTED_MANIFEST_SHA256: [u8; 32] = [0; 32];
const EXPECTED_PUBLISHER_CERT_SHA256: [u8; 32] = [0; 32];

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProvisionManifest {
    schema: String,
    package_candidate_id: String,
    installer_transaction_evidence_id: String,
    actor_image_sha256: String,
    wire_profile_id: String,
    wire_semantic_manifest_sha256: String,
    wire_artifact_index_sha256: String,
    provision_record_layout_sha256: String,
    machine_data_directory: String,
    provision_record_name: String,
    machine_actor_record_name: String,
}

fn decode_sha256(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    let mut decoded = [0_u8; 32];
    for (index, byte) in decoded.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(decoded)
}

fn validate_manifest(bytes: &[u8], actor_digest: [u8; 32]) -> Option<[u8; 32]> {
    if actor_digest == [0; 32]
        || bytes.is_empty()
        || bytes.len() > MAX_MANIFEST_BYTES
        || bytes.starts_with(&[0xef, 0xbb, 0xbf])
    {
        return None;
    }
    let manifest: ProvisionManifest = serde_json::from_slice(bytes).ok()?;
    if serde_json::to_vec(&manifest).ok()? != bytes
        || manifest.schema != MANIFEST_SCHEMA
        || manifest.package_candidate_id.is_empty()
        || manifest.package_candidate_id.len() > 64
        || !manifest
            .package_candidate_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        || manifest.package_candidate_id == "."
        || manifest.package_candidate_id == ".."
        || manifest.installer_transaction_evidence_id.len() != 32
        || !manifest
            .installer_transaction_evidence_id
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        || manifest
            .installer_transaction_evidence_id
            .bytes()
            .all(|byte| byte == b'0')
        || decode_sha256(&manifest.actor_image_sha256)? != actor_digest
        || manifest.wire_profile_id != WIRE_PROFILE_ID
        || manifest.wire_semantic_manifest_sha256 != WIRE_SEMANTIC_MANIFEST_SHA256
        || manifest.wire_artifact_index_sha256 != WIRE_ARTIFACT_INDEX_SHA256
        || manifest.provision_record_layout_sha256 != PROVISION_RECORD_LAYOUT_SHA256
        || manifest.machine_data_directory != MACHINE_DATA_DIRECTORY
        || manifest.provision_record_name != PROVISION_RECORD_NAME
        || manifest.machine_actor_record_name != MACHINE_ACTOR_RECORD_NAME
    {
        return None;
    }
    Some(Sha256::digest(bytes).into())
}

fn require_configured_authority() -> Result<(), &'static str> {
    if EXPECTED_MANIFEST_SHA256 == [0; 32] || EXPECTED_PUBLISHER_CERT_SHA256 == [0; 32] {
        return Err("PROVISION_MANIFEST_AUTHORITY_UNCONFIGURED");
    }
    Ok(())
}

#[cfg(any(target_os = "windows", test))]
fn service_binary_command(path: &[u16]) -> Option<Vec<u16>> {
    if path.is_empty() || path.iter().any(|unit| *unit < 0x20 || *unit == b'"' as u16) {
        return None;
    }
    let mut command = Vec::with_capacity(path.len() + SERVICE_ARGUMENT.len() + 5);
    command.push(b'"' as u16);
    command.extend_from_slice(path);
    command.extend("\" ".encode_utf16());
    command.extend(SERVICE_ARGUMENT.encode_utf16());
    command.push(0);
    Some(command)
}

pub fn run_system_provision_handshake() -> Result<(), String> {
    require_configured_authority()?;
    #[cfg(target_os = "windows")]
    {
        platform::coordinate()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("LocalSystem provision handshake is Windows-only".into())
    }
}

pub fn run_system_provision_service() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        platform::dispatch()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("LocalSystem provision service is Windows-only".into())
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use sha2::{Digest, Sha256};
    use std::{
        fs::File,
        io::{Read, Seek, SeekFrom},
        mem::{size_of, size_of_val},
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
        ptr, slice, thread,
        time::{Duration, Instant},
    };

    use windows::{
        core::{w, HRESULT, PCWSTR, PSTR, PWSTR},
        Win32::{
            Foundation::{
                CRYPT_E_NO_SIGNER, ERROR_SERVICE_EXISTS, ERROR_SERVICE_SPECIFIC_ERROR, HANDLE,
                NO_ERROR,
            },
            Security::Cryptography::{
                szOID_PKIX_KP_CODE_SIGNING, CertFreeCertificateChain, CertFreeCertificateContext,
                CertGetCertificateChain, CertGetCertificateContextProperty,
                CertVerifyCertificateChainPolicy, CryptVerifyDetachedMessageSignature,
                CERT_CHAIN_PARA, CERT_CHAIN_POLICY_AUTHENTICODE, CERT_CHAIN_POLICY_PARA,
                CERT_CHAIN_POLICY_STATUS, CERT_CONTEXT, CERT_SHA256_HASH_PROP_ID,
                CRYPT_VERIFY_MESSAGE_PARA, CTL_USAGE, PKCS_7_ASN_ENCODING, USAGE_MATCH_TYPE_AND,
                X509_ASN_ENCODING,
            },
            System::{
                RemoteDesktop::{
                    WTSActive, WTSConnected, WTSDisconnected, WTSEnumerateSessionsW, WTSFreeMemory,
                    WTSGetActiveConsoleSessionId, WTSQuerySessionInformationW, WTSQueryUserToken,
                    WTSUserName, WTS_CURRENT_SERVER_HANDLE, WTS_SESSION_INFOW,
                },
                Services::{
                    CreateServiceW, DeleteService, OpenSCManagerW, OpenServiceW,
                    QueryServiceConfigW, QueryServiceStatusEx, RegisterServiceCtrlHandlerW,
                    SetServiceStatus, StartServiceCtrlDispatcherW, StartServiceW,
                    QUERY_SERVICE_CONFIGW, SC_HANDLE, SC_MANAGER_CONNECT,
                    SC_MANAGER_CREATE_SERVICE, SC_STATUS_PROCESS_INFO, SERVICE_DEMAND_START,
                    SERVICE_ERROR_NORMAL, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS,
                    SERVICE_RUNNING, SERVICE_START, SERVICE_START_PENDING, SERVICE_STATUS,
                    SERVICE_STATUS_PROCESS, SERVICE_STOPPED, SERVICE_STOP_PENDING,
                    SERVICE_TABLE_ENTRYW, SERVICE_WIN32_OWN_PROCESS,
                },
            },
        },
    };

    use super::{
        require_configured_authority, service_binary_command, validate_manifest, ACTOR_IMAGE_NAME,
        EXPECTED_MANIFEST_SHA256, EXPECTED_PUBLISHER_CERT_SHA256, MANIFEST_NAME,
        MANIFEST_SIGNATURE_NAME, MAX_ACTOR_BYTES, MAX_MANIFEST_BYTES, MAX_SIGNATURE_BYTES,
    };
    use crate::machine_storage::{
        current_process_is_local_system, token_sid_digest, FreshProvisionObservation,
        InstallFileEvidence, ProtectedInstall, ProvisionMachineGate,
    };

    const SERVICE_DELETE: u32 = 0x0001_0000;
    const SERVICE_ACCESS: u32 =
        SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS | SERVICE_START | SERVICE_DELETE;
    const MAX_CONFIG_BYTES: usize = 8 * 1024;
    const MAX_SESSIONS: u32 = 64;
    const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
    const IDENTITY_NOT_SYSTEM: u32 = 1;
    const CONSOLE_SESSION_UNPROVEN: u32 = 2;
    const DESIGNATED_SID_UNPROVEN: u32 = 3;
    const MANIFEST_AUTHORITY_UNCONFIGURED: u32 = 4;
    const MANIFEST_UNPROVEN: u32 = 5;
    const SIGNER_UNPROVEN: u32 = 6;
    const PROVISION_ANCHOR_NOT_IMPLEMENTED: u32 = 7;
    const INSTALL_ANCHOR_UNPROVEN: u32 = 8;
    const PROCESS_IMAGE_NAME_UNPROVEN: u32 = 9;
    const FRESH_PROVISION_OBSERVATION_UNPROVEN: u32 = 10;
    const MACHINE_GATE_UNPROVEN: u32 = 11;
    const SERVICE_NAME: PCWSTR = w!("DisplayDeckProvisionV1");
    const SERVICE_DISPLAY_NAME: PCWSTR = w!("DisplayDeck Provision V1");

    pub(super) fn coordinate() -> Result<(), String> {
        let install = ProtectedInstall::open()
            .ok_or_else(|| "PROVISION_INSTALL_ANCHOR_UNPROVEN".to_string())?;
        // Retain every parent and package file until the SCM handshake finishes.
        let _package = verify_install_package(&install)
            .map_err(|code| format!("provision package evidence failed: {code}"))?;
        let command = service_binary_command(install.actor_path())
            .ok_or_else(|| "fixed actor image cannot form a service command".to_string())?;

        let manager = ServiceHandle(
            unsafe {
                OpenSCManagerW(
                    PCWSTR::null(),
                    PCWSTR::null(),
                    SC_MANAGER_CONNECT | SC_MANAGER_CREATE_SERVICE,
                )
            }
            .map_err(|error| format!("open local service manager: {error}"))?,
        );
        let service = match create_service(manager.0, &command) {
            Ok(handle) => ServiceHandle(handle),
            Err(error) if error.code() == HRESULT::from_win32(ERROR_SERVICE_EXISTS.0) => {
                ServiceHandle(
                    unsafe { OpenServiceW(manager.0, SERVICE_NAME, SERVICE_ACCESS) }
                        .map_err(|error| format!("open retained provision service: {error}"))?,
                )
            }
            Err(error) => return Err(format!("create provision service: {error}")),
        };

        if !service_config_matches(service.0, &command)? {
            return Err("retained provision service configuration mismatch".into());
        }
        if query_status(service.0)?.dwCurrentState != SERVICE_STOPPED {
            return Err("retained provision service is not stopped".into());
        }
        if !install.revalidate() || !install.process_image_name_matches() {
            return Err("PROVISION_INSTALL_ANCHOR_CHANGED".into());
        }
        unsafe { StartServiceW(service.0, None) }
            .map_err(|error| format!("start provision service: {error}"))?;

        let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
        loop {
            let status = query_status(service.0)?;
            match status.dwCurrentState {
                SERVICE_STOPPED => {
                    if status.dwWin32ExitCode != NO_ERROR.0 || status.dwServiceSpecificExitCode != 0
                    {
                        return Err(format!(
                            "provision service identity handshake failed: {}/{}",
                            status.dwWin32ExitCode, status.dwServiceSpecificExitCode
                        ));
                    }
                    unsafe { DeleteService(service.0) }
                        .map_err(|error| format!("delete completed provision service: {error}"))?;
                    return Ok(());
                }
                SERVICE_START_PENDING | SERVICE_RUNNING | SERVICE_STOP_PENDING => {}
                _ => return Err("provision service entered an unexpected state".into()),
            }
            if Instant::now() >= deadline {
                return Err("provision service identity handshake timed out".into());
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn create_service(manager: SC_HANDLE, command: &[u16]) -> windows::core::Result<SC_HANDLE> {
        unsafe {
            CreateServiceW(
                manager,
                SERVICE_NAME,
                SERVICE_DISPLAY_NAME,
                SERVICE_ACCESS,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_DEMAND_START,
                SERVICE_ERROR_NORMAL,
                PCWSTR(command.as_ptr()),
                PCWSTR::null(),
                None,
                PCWSTR::null(),
                PCWSTR::null(),
                PCWSTR::null(),
            )
        }
    }

    fn service_config_matches(
        service: SC_HANDLE,
        expected_command: &[u16],
    ) -> Result<bool, String> {
        let mut storage = [0_usize; MAX_CONFIG_BYTES / size_of::<usize>()];
        let mut needed = 0_u32;
        unsafe {
            QueryServiceConfigW(
                service,
                Some(storage.as_mut_ptr().cast()),
                u32::try_from(size_of_val(&storage)).expect("service config buffer fits in u32"),
                &mut needed,
            )
        }
        .map_err(|error| format!("query provision service configuration: {error}"))?;
        if usize::try_from(needed).unwrap_or(usize::MAX) > size_of_val(&storage) {
            return Ok(false);
        }
        let config = unsafe {
            // SAFETY: the aligned fixed buffer contains the successful QUERY_SERVICE_CONFIGW.
            &*storage.as_ptr().cast::<QUERY_SERVICE_CONFIGW>()
        };
        let command = unsafe { buffered_string(&storage, config.lpBinaryPathName) };
        let group = unsafe { buffered_string(&storage, config.lpLoadOrderGroup) };
        let dependencies = unsafe { buffered_string(&storage, config.lpDependencies) };
        let account = unsafe { buffered_string(&storage, config.lpServiceStartName) };
        let display = unsafe { buffered_string(&storage, config.lpDisplayName) };
        Ok(config.dwServiceType == SERVICE_WIN32_OWN_PROCESS
            && config.dwStartType == SERVICE_DEMAND_START
            && config.dwErrorControl == SERVICE_ERROR_NORMAL
            && config.dwTagId == 0
            && command.as_deref() == expected_command.strip_suffix(&[0])
            && group.as_deref() == Some(&[])
            && dependencies.as_deref() == Some(&[])
            && account
                .and_then(|value| String::from_utf16(&value).ok())
                .is_some_and(|value| value.eq_ignore_ascii_case("LocalSystem"))
            && display.as_deref()
                == Some(
                    &"DisplayDeck Provision V1"
                        .encode_utf16()
                        .collect::<Vec<_>>(),
                ))
    }

    unsafe fn buffered_string(storage: &[usize], value: PWSTR) -> Option<Vec<u16>> {
        if value.is_null() {
            return Some(Vec::new());
        }
        let start = storage.as_ptr() as usize;
        let end = start.checked_add(size_of_val(storage))?;
        let pointer = value.as_ptr() as usize;
        if pointer < start || pointer >= end || pointer % size_of::<u16>() != 0 {
            return None;
        }
        let units = (end - pointer) / size_of::<u16>();
        let value = unsafe {
            // SAFETY: the pointer was proven inside the live query buffer.
            slice::from_raw_parts(value.as_ptr(), units)
        };
        let terminator = value.iter().position(|unit| *unit == 0)?;
        Some(value[..terminator].to_vec())
    }

    fn query_status(service: SC_HANDLE) -> Result<SERVICE_STATUS_PROCESS, String> {
        let mut status = SERVICE_STATUS_PROCESS::default();
        let mut needed = 0_u32;
        let bytes = unsafe {
            // SAFETY: `status` is writable for exactly its byte size during the call.
            slice::from_raw_parts_mut(
                ptr::addr_of_mut!(status).cast::<u8>(),
                size_of::<SERVICE_STATUS_PROCESS>(),
            )
        };
        unsafe { QueryServiceStatusEx(service, SC_STATUS_PROCESS_INFO, Some(bytes), &mut needed) }
            .map_err(|error| format!("query provision service status: {error}"))?;
        if needed != u32::try_from(size_of::<SERVICE_STATUS_PROCESS>()).unwrap() {
            return Err("provision service status size mismatch".into());
        }
        Ok(status)
    }

    pub(super) fn dispatch() -> Result<(), String> {
        let table = [
            SERVICE_TABLE_ENTRYW {
                lpServiceName: PWSTR(SERVICE_NAME.as_ptr().cast_mut()),
                lpServiceProc: Some(service_main),
            },
            SERVICE_TABLE_ENTRYW::default(),
        ];
        unsafe { StartServiceCtrlDispatcherW(table.as_ptr()) }
            .map_err(|error| format!("connect provision service dispatcher: {error}"))
    }

    unsafe extern "system" fn service_main(_: u32, _: *mut PWSTR) {
        let Ok(handle) =
            (unsafe { RegisterServiceCtrlHandlerW(SERVICE_NAME, Some(service_handler)) })
        else {
            return;
        };
        if set_status(handle, SERVICE_START_PENDING, NO_ERROR.0, 0).is_err() {
            return;
        }
        let failure = match provision_handshake() {
            Ok(()) => 0,
            Err(code) => code,
        };
        if failure == 0 {
            if set_status(handle, SERVICE_RUNNING, NO_ERROR.0, 0).is_err() {
                return;
            }
            let _ = set_status(handle, SERVICE_STOPPED, NO_ERROR.0, 0);
        } else {
            let _ = set_status(
                handle,
                SERVICE_STOPPED,
                ERROR_SERVICE_SPECIFIC_ERROR.0,
                failure,
            );
        }
    }

    unsafe extern "system" fn service_handler(_: u32) {}

    fn set_status(
        handle: windows::Win32::System::Services::SERVICE_STATUS_HANDLE,
        state: windows::Win32::System::Services::SERVICE_STATUS_CURRENT_STATE,
        win32_exit: u32,
        specific_exit: u32,
    ) -> windows::core::Result<()> {
        let status = SERVICE_STATUS {
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: state,
            dwControlsAccepted: 0,
            dwWin32ExitCode: win32_exit,
            dwServiceSpecificExitCode: specific_exit,
            dwCheckPoint: 0,
            dwWaitHint: 0,
        };
        unsafe { SetServiceStatus(handle, &status) }
    }

    fn provision_handshake() -> Result<(), u32> {
        let (owner_token, owner_sid_digest) = identity_handshake()?;
        manifest_authority_handshake(HANDLE(owner_token.as_raw_handle()), owner_sid_digest)
    }

    fn identity_handshake() -> Result<(OwnedHandle, [u8; 32]), u32> {
        if !current_process_is_local_system() {
            return Err(IDENTITY_NOT_SYSTEM);
        }
        let console = single_interactive_console().ok_or(CONSOLE_SESSION_UNPROVEN)?;
        let mut token = HANDLE::default();
        // SAFETY: WTS returns an owned token for the observed active console session.
        unsafe { WTSQueryUserToken(console, &mut token) }.map_err(|_| DESIGNATED_SID_UNPROVEN)?;
        // SAFETY: transfer this newly returned handle once; all paths close it via Drop.
        let token = unsafe { OwnedHandle::from_raw_handle(token.0) };
        let digest =
            token_sid_digest(HANDLE(token.as_raw_handle())).ok_or(DESIGNATED_SID_UNPROVEN)?;
        Ok((token, digest))
    }

    fn manifest_authority_handshake(
        owner_token: HANDLE,
        owner_sid_digest: [u8; 32],
    ) -> Result<(), u32> {
        if require_configured_authority().is_err() {
            return Err(MANIFEST_AUTHORITY_UNCONFIGURED);
        }
        let install = ProtectedInstall::open().ok_or(INSTALL_ANCHOR_UNPROVEN)?;
        let _package = verify_install_package(&install)?;

        let _gate = ProvisionMachineGate::acquire(owner_token, owner_sid_digest)
            .ok_or(MACHINE_GATE_UNPROVEN)?;
        let fresh = FreshProvisionObservation::observe(&_gate, owner_token, owner_sid_digest)
            .ok_or(FRESH_PROVISION_OBSERVATION_UNPROVEN)?;
        if !fresh.reobserve() || !install.revalidate() {
            return Err(FRESH_PROVISION_OBSERVATION_UNPROVEN);
        }

        // ponytail: evidence checks only. Loaded-process image binding, certificate
        // lifecycle policy and Candidate 04 writer must precede any grant.
        // Gate ownership and absence observations are not atomic create authority.
        Err(PROVISION_ANCHOR_NOT_IMPLEMENTED)
    }

    fn verify_install_package(install: &ProtectedInstall) -> Result<[HeldFile; 3], u32> {
        require_configured_authority().map_err(|_| MANIFEST_AUTHORITY_UNCONFIGURED)?;
        if !install.process_image_name_matches() {
            return Err(PROCESS_IMAGE_NAME_UNPROVEN);
        }
        let mut actor =
            HeldFile::open(install, ACTOR_IMAGE_NAME, MAX_ACTOR_BYTES).ok_or(MANIFEST_UNPROVEN)?;
        let actor_digest = actor.digest(install).ok_or(MANIFEST_UNPROVEN)?;
        let mut manifest_file = HeldFile::open(install, MANIFEST_NAME, MAX_MANIFEST_BYTES as u64)
            .ok_or(MANIFEST_UNPROVEN)?;
        let manifest = manifest_file.read(install).ok_or(MANIFEST_UNPROVEN)?;
        let mut signature_file =
            HeldFile::open(install, MANIFEST_SIGNATURE_NAME, MAX_SIGNATURE_BYTES as u64)
                .ok_or(MANIFEST_UNPROVEN)?;
        let signature = signature_file.read(install).ok_or(MANIFEST_UNPROVEN)?;
        let manifest_digest =
            validate_manifest(&manifest, actor_digest).ok_or(MANIFEST_UNPROVEN)?;
        if manifest_digest != EXPECTED_MANIFEST_SHA256 {
            return Err(MANIFEST_UNPROVEN);
        }
        verify_detached_signature(&manifest, &signature, EXPECTED_PUBLISHER_CERT_SHA256)
            .map_err(|_| SIGNER_UNPROVEN)?;
        if actor.digest(install) != Some(actor_digest)
            || manifest_file.read(install).as_deref() != Some(manifest.as_slice())
            || signature_file.read(install).as_deref() != Some(signature.as_slice())
            || !install.revalidate()
            || !install.process_image_name_matches()
        {
            return Err(MANIFEST_UNPROVEN);
        }

        Ok([actor, manifest_file, signature_file])
    }

    struct HeldFile {
        file: File,
        evidence: InstallFileEvidence,
        name: &'static str,
        length: u64,
    }

    impl HeldFile {
        fn open(install: &ProtectedInstall, name: &'static str, maximum: u64) -> Option<Self> {
            let file = install.open_file(name)?;
            let length = file.metadata().ok()?.len();
            if length == 0 || length > maximum {
                return None;
            }
            let evidence = install.file_evidence(&file, name, length)?;
            Some(Self {
                file,
                evidence,
                name,
                length,
            })
        }

        fn unchanged(&self, install: &ProtectedInstall) -> bool {
            install
                .file_evidence(&self.file, self.name, self.length)
                .as_ref()
                == Some(&self.evidence)
        }

        fn read(&mut self, install: &ProtectedInstall) -> Option<Vec<u8>> {
            self.file.seek(SeekFrom::Start(0)).ok()?;
            let mut bytes = Vec::with_capacity(usize::try_from(self.length).ok()?);
            self.file
                .by_ref()
                .take(self.length.checked_add(1)?)
                .read_to_end(&mut bytes)
                .ok()?;
            (bytes.len() as u64 == self.length && self.unchanged(install)).then_some(bytes)
        }

        fn digest(&mut self, install: &ProtectedInstall) -> Option<[u8; 32]> {
            self.file.seek(SeekFrom::Start(0)).ok()?;
            let mut digest = Sha256::new();
            let mut total = 0_u64;
            let mut buffer = [0_u8; 16 * 1024];
            loop {
                let count = self.file.read(&mut buffer).ok()?;
                if count == 0 {
                    break;
                }
                total = total.checked_add(count as u64)?;
                if total > self.length {
                    return None;
                }
                digest.update(&buffer[..count]);
            }
            (total == self.length && self.unchanged(install)).then(|| digest.finalize().into())
        }
    }

    fn verify_detached_signature(
        manifest: &[u8],
        signature: &[u8],
        expected_signer: [u8; 32],
    ) -> Result<(), ()> {
        if manifest.is_empty()
            || manifest.len() > MAX_MANIFEST_BYTES
            || signature.is_empty()
            || signature.len() > MAX_SIGNATURE_BYTES
            || expected_signer == [0; 32]
        {
            return Err(());
        }
        let verify = CRYPT_VERIFY_MESSAGE_PARA {
            cbSize: u32::try_from(size_of::<CRYPT_VERIFY_MESSAGE_PARA>()).map_err(|_| ())?,
            dwMsgAndCertEncodingType: X509_ASN_ENCODING.0 | PKCS_7_ASN_ENCODING.0,
            ..Default::default()
        };
        let fragments = [manifest.as_ptr()];
        let lengths = [u32::try_from(manifest.len()).map_err(|_| ())?];
        let mut signer = ptr::null_mut();
        // SAFETY: bounded buffers and single-fragment arrays remain live for the call;
        // signer is an initialized out-pointer, owned below on successful verification.
        unsafe {
            CryptVerifyDetachedMessageSignature(
                &verify,
                0,
                signature,
                1,
                fragments.as_ptr(),
                lengths.as_ptr(),
                Some(&mut signer),
            )
        }
        .map_err(|_| ())?;
        if signer.is_null() {
            return Err(());
        }
        let signer = CertificateContext(signer);
        // SAFETY: same live buffers; signer index 1 is queried without taking a certificate.
        match unsafe {
            CryptVerifyDetachedMessageSignature(
                &verify,
                1,
                signature,
                1,
                fragments.as_ptr(),
                lengths.as_ptr(),
                None,
            )
        } {
            Err(error) if error.code() == CRYPT_E_NO_SIGNER => {}
            _ => return Err(()),
        }

        let mut signer_digest = [0_u8; 32];
        let mut signer_digest_length = u32::try_from(signer_digest.len()).map_err(|_| ())?;
        // SAFETY: signer owns a valid context; the output is exactly the advertised 32 bytes.
        unsafe {
            CertGetCertificateContextProperty(
                signer.0,
                CERT_SHA256_HASH_PROP_ID,
                Some(signer_digest.as_mut_ptr().cast()),
                &mut signer_digest_length,
            )
        }
        .map_err(|_| ())?;
        if signer_digest_length != 32 || signer_digest != expected_signer {
            return Err(());
        }

        let mut usage_identifier = PSTR(szOID_PKIX_KP_CODE_SIGNING.0.cast_mut());
        let mut chain_parameters = CERT_CHAIN_PARA {
            cbSize: u32::try_from(size_of::<CERT_CHAIN_PARA>()).map_err(|_| ())?,
            ..Default::default()
        };
        chain_parameters.RequestedUsage.dwType = USAGE_MATCH_TYPE_AND;
        chain_parameters.RequestedUsage.Usage = CTL_USAGE {
            cUsageIdentifier: 1,
            rgpszUsageIdentifier: &mut usage_identifier,
        };
        // SAFETY: successful verification returned this context, retained until after chain use.
        let additional_store = unsafe { (*signer.0).hCertStore };
        let mut chain = ptr::null_mut();
        // SAFETY: context, store and usage OID buffers outlive the call; chain is an out-pointer.
        unsafe {
            CertGetCertificateChain(
                None,
                signer.0,
                None,
                (!additional_store.is_invalid()).then_some(additional_store),
                &chain_parameters,
                0,
                None,
                &mut chain,
            )
        }
        .map_err(|_| ())?;
        if chain.is_null() {
            return Err(());
        }
        let chain = CertificateChain(chain);
        let policy = CERT_CHAIN_POLICY_PARA {
            cbSize: u32::try_from(size_of::<CERT_CHAIN_POLICY_PARA>()).map_err(|_| ())?,
            ..Default::default()
        };
        let mut status = CERT_CHAIN_POLICY_STATUS {
            cbSize: u32::try_from(size_of::<CERT_CHAIN_POLICY_STATUS>()).map_err(|_| ())?,
            ..Default::default()
        };
        // SAFETY: chain owns a valid context and both policy structures have correct sizes.
        let verified = unsafe {
            CertVerifyCertificateChainPolicy(
                CERT_CHAIN_POLICY_AUTHENTICODE,
                chain.0,
                &policy,
                &mut status,
            )
        };
        (verified.as_bool() && status.dwError == 0)
            .then_some(())
            .ok_or(())
    }

    struct CertificateContext(*mut CERT_CONTEXT);
    impl Drop for CertificateContext {
        fn drop(&mut self) {
            // SAFETY: owns the single context returned by successful detached verification.
            let _ = unsafe { CertFreeCertificateContext(Some(self.0)) };
        }
    }

    struct CertificateChain(*mut windows::Win32::Security::Cryptography::CERT_CHAIN_CONTEXT);
    impl Drop for CertificateChain {
        fn drop(&mut self) {
            // SAFETY: owns the chain returned by successful CertGetCertificateChain.
            unsafe { CertFreeCertificateChain(self.0) };
        }
    }

    fn single_interactive_console() -> Option<u32> {
        let console = unsafe { WTSGetActiveConsoleSessionId() };
        if console == u32::MAX {
            return None;
        }
        let mut sessions = ptr::null_mut();
        let mut count = 0_u32;
        unsafe {
            WTSEnumerateSessionsW(
                Some(WTS_CURRENT_SERVER_HANDLE),
                0,
                1,
                &mut sessions,
                &mut count,
            )
        }
        .ok()?;
        let result = inspect_sessions(sessions, count, console);
        if !sessions.is_null() {
            unsafe { WTSFreeMemory(sessions.cast()) };
        }
        result
    }

    fn inspect_sessions(sessions: *mut WTS_SESSION_INFOW, count: u32, console: u32) -> Option<u32> {
        if count == 0 || count > MAX_SESSIONS || sessions.is_null() {
            return None;
        }
        let sessions = unsafe {
            // SAFETY: successful enumeration returned `count` initialized entries.
            slice::from_raw_parts(sessions, usize::try_from(count).ok()?)
        };
        let mut interactive = Vec::new();
        let mut console_active = false;
        for session in sessions {
            if session.SessionId == console && session.State == WTSActive {
                console_active = true;
            }
            if (session.State == WTSActive
                || session.State == WTSConnected
                || session.State == WTSDisconnected)
                && session_has_user(session.SessionId)?
            {
                interactive.push(session.SessionId);
            }
        }
        (console_active && interactive.as_slice() == [console]).then_some(console)
    }

    fn session_has_user(session: u32) -> Option<bool> {
        let mut buffer = PWSTR::null();
        let mut bytes = 0_u32;
        if unsafe {
            WTSQuerySessionInformationW(
                Some(WTS_CURRENT_SERVER_HANDLE),
                session,
                WTSUserName,
                &mut buffer,
                &mut bytes,
            )
        }
        .is_err()
        {
            if !buffer.is_null() {
                unsafe { WTSFreeMemory(buffer.as_ptr().cast()) };
            }
            return None;
        }
        let result = (|| {
            let bytes = usize::try_from(bytes).ok()?;
            if bytes < size_of::<u16>() || bytes % size_of::<u16>() != 0 || buffer.is_null() {
                return Some(false);
            }
            let value = unsafe {
                // SAFETY: WTS returned exactly `bytes` readable bytes.
                slice::from_raw_parts(buffer.as_ptr(), bytes / size_of::<u16>())
            };
            let terminator = value.iter().position(|unit| *unit == 0)?;
            Some(
                terminator != 0
                    && value[terminator..].iter().all(|unit| *unit == 0)
                    && String::from_utf16(&value[..terminator]).is_ok(),
            )
        })();
        if !buffer.is_null() {
            unsafe { WTSFreeMemory(buffer.as_ptr().cast()) };
        }
        result
    }

    struct ServiceHandle(SC_HANDLE);
    impl Drop for ServiceHandle {
        fn drop(&mut self) {
            let _ = unsafe { windows::Win32::System::Services::CloseServiceHandle(self.0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn service_command_has_one_fixed_quoted_argument() {
        let path = r"C:\Program Files\DisplayDeck\displaydeck-actor.exe"
            .encode_utf16()
            .collect::<Vec<_>>();
        let command = service_binary_command(&path).unwrap();
        assert_eq!(
            String::from_utf16(&command[..command.len() - 1]).unwrap(),
            r#""C:\Program Files\DisplayDeck\displaydeck-actor.exe" --provision-service"#
        );
        assert!(service_binary_command(&[b'C' as u16, b'"' as u16]).is_none());
        assert!(service_binary_command(&[b'C' as u16, 0]).is_none());
    }

    #[test]
    fn manifest_is_canonical_exact_and_pins_remain_unconfigured() {
        let actor_digest = [0xab; 32];
        let manifest = ProvisionManifest {
            schema: MANIFEST_SCHEMA.into(),
            package_candidate_id: "displaydeck-mutation-candidate-01".into(),
            installer_transaction_evidence_id: "1234567890abcdef1234567890abcdef".into(),
            actor_image_sha256: encode_hex(&actor_digest),
            wire_profile_id: WIRE_PROFILE_ID.into(),
            wire_semantic_manifest_sha256: WIRE_SEMANTIC_MANIFEST_SHA256.into(),
            wire_artifact_index_sha256: WIRE_ARTIFACT_INDEX_SHA256.into(),
            provision_record_layout_sha256: PROVISION_RECORD_LAYOUT_SHA256.into(),
            machine_data_directory: MACHINE_DATA_DIRECTORY.into(),
            provision_record_name: PROVISION_RECORD_NAME.into(),
            machine_actor_record_name: MACHINE_ACTOR_RECORD_NAME.into(),
        };
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(
            validate_manifest(&bytes, actor_digest),
            Some(Sha256::digest(&bytes).into())
        );
        assert_eq!(validate_manifest(&bytes, [0; 32]), None);
        assert_eq!(validate_manifest(&bytes, [0xac; 32]), None);
        let text = std::str::from_utf8(&bytes).unwrap();
        for changed in [
            text.replacen("\"schema\":", "\"schema\":\"duplicate\",\"schema\":", 1),
            text.replace(WIRE_PROFILE_ID, "DD-FR-002-WIRE-PROFILE-V1-CANDIDATE-03"),
            text.replace(PROVISION_RECORD_NAME, "arbitrary-record"),
            text.replace("\"schema\"", "\"\\u0073chema\""),
            format!("\u{feff}{text}"),
            "x".repeat(MAX_MANIFEST_BYTES + 1),
        ] {
            assert_eq!(validate_manifest(changed.as_bytes(), actor_digest), None);
        }

        let mut noncanonical = bytes.clone();
        noncanonical.push(b'\n');
        assert_eq!(validate_manifest(&noncanonical, actor_digest), None);
        let unknown = String::from_utf8(bytes.clone())
            .unwrap()
            .replace('}', ",\"extra\":1}");
        assert_eq!(validate_manifest(unknown.as_bytes(), actor_digest), None);
        let uppercase = String::from_utf8(bytes).unwrap().replace(
            &encode_hex(&actor_digest),
            &encode_hex(&actor_digest).to_uppercase(),
        );
        assert_eq!(validate_manifest(uppercase.as_bytes(), actor_digest), None);
        assert_eq!(EXPECTED_MANIFEST_SHA256, [0; 32]);
        assert_eq!(EXPECTED_PUBLISHER_CERT_SHA256, [0; 32]);
        assert_eq!(
            require_configured_authority(),
            Err("PROVISION_MANIFEST_AUTHORITY_UNCONFIGURED")
        );
        // Exercises the actual coordinator entry point; the guard precedes all SCM calls.
        assert_eq!(
            run_system_provision_handshake(),
            Err("PROVISION_MANIFEST_AUTHORITY_UNCONFIGURED".into())
        );
    }
}
