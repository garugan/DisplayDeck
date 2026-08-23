use std::{
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};

use displaydeck_safety::{
    current_tick_ms, random_id, ActorStatus, SafetyEngine, SafetyStatus, WatchdogCommand,
    WatchdogStart, WorkerGo, WorkerGrant, WorkerHello, WorkerIdentity, WorkerResult, WorkerRole,
};
use sha2::{Digest, Sha256};

#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;
#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{FILETIME, HANDLE},
    System::Threading::{GetCurrentProcess, GetProcessTimes},
};

fn main() {
    let result = if std::env::args().nth(1).as_deref() == Some("--worker") {
        run_worker()
    } else {
        run_watchdog()
    };
    if let Err(error) = result {
        eprintln!("displaydeck-actor: {error}");
        std::process::exit(1);
    }
}

fn run_worker() -> Result<(), String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("resolve worker image: {error}"))?;
    let identity = WorkerIdentity {
        pid: std::process::id(),
        process_creation_time: current_process_creation_time()?,
        image_digest: file_digest(&executable)?,
        role: WorkerRole::FakeOneShot,
        process_nonce: random_id()?,
    };
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    serde_json::to_writer(
        &mut stdout,
        &WorkerHello {
            schema_version: 1,
            identity: identity.clone(),
        },
    )
    .map_err(|error| format!("write worker HELLO: {error}"))?;
    stdout
        .write_all(b"\n")
        .and_then(|_| stdout.flush())
        .map_err(|error| format!("flush worker HELLO: {error}"))?;

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("read worker GO: {error}"))?;
    let go: WorkerGo =
        serde_json::from_str(&input).map_err(|error| format!("parse worker GO: {error}"))?;
    if go.schema_version != 1
        || go.expected_identity != identity
        || !go.grant.fence.is_valid()
        || go.grant.machine_epoch == 0
        || go.grant.operation_nonce == [0; 16]
        || go.grant.sequence == 0
    {
        return Err("invalid worker authority".into());
    }
    serde_json::to_writer(
        &mut stdout,
        &WorkerResult {
            schema_version: 1,
            identity,
            operation: go.grant.operation,
            operation_nonce: go.grant.operation_nonce,
            sequence: go.grant.sequence,
            succeeded: true,
        },
    )
    .map_err(|error| format!("write worker result: {error}"))?;
    Ok(())
}

fn run_watchdog() -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    if sender.send(Some(line)).is_err() {
                        return;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = sender.send(None);
    });
    let first = receiver
        .recv()
        .map_err(|_| "watchdog control pipe closed before start".to_string())?
        .ok_or_else(|| "missing watchdog start frame".to_string())?;
    let mut start: WatchdogStart = serde_json::from_str(&first)
        .map_err(|error| format!("parse watchdog start frame: {error}"))?;
    if start.schema_version != 1 || !start.input.fence.is_valid() {
        return Err("invalid watchdog start authority".into());
    }
    let now = current_tick_ms();
    let remaining = start
        .input
        .confirmation_deadline_tick_ms
        .checked_sub(now)
        .ok_or_else(|| "watchdog start deadline expired".to_string())?;
    if !(1..=15_000).contains(&remaining) {
        return Err("watchdog start deadline is outside the 15-second bound".into());
    }
    start.input.now_tick_ms = now;
    let storage_dir = validate_storage_dir(&start.storage_dir)?;
    let status_path = storage_dir.join("status.json");
    let (mut engine, apply_grant) =
        SafetyEngine::begin(&storage_dir, start.input, start.fault_plan)
            .map_err(|error| error.to_string())?;
    let apply_succeeded = run_one_shot_worker(&apply_grant)?;
    if let Some(restore) = engine
        .complete_worker(&apply_grant, apply_succeeded)
        .map_err(|error| error.to_string())?
    {
        finish_restore(&mut engine, &restore)?;
    }
    write_status(&status_path, &engine, None)?;

    loop {
        let now = current_tick_ms();
        let wait = Duration::from_millis(50);
        match receiver.recv_timeout(wait) {
            Ok(Some(frame)) => {
                let command: WatchdogCommand = serde_json::from_str(&frame)
                    .map_err(|error| format!("parse watchdog command: {error}"))?;
                match command {
                    WatchdogCommand::Confirm { mut command } => {
                        command.now_tick_ms = current_tick_ms();
                        match engine.confirm(command) {
                            Ok(_) => write_status(&status_path, &engine, None)?,
                            Err(error) => {
                                write_status(&status_path, &engine, Some(error.to_string()))?;
                            }
                        }
                        if matches!(engine.status(), SafetyStatus::Terminal(_)) {
                            return Ok(());
                        }
                    }
                    WatchdogCommand::Revert { command } => {
                        let restore = engine
                            .manual_revert(command)
                            .map_err(|error| error.to_string())?;
                        finish_restore(&mut engine, &restore)?;
                        write_status(&status_path, &engine, None)?;
                        return Ok(());
                    }
                }
            }
            Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                let restore = engine.parent_loss().map_err(|error| error.to_string())?;
                finish_restore(&mut engine, &restore)?;
                write_status(&status_path, &engine, None)?;
                return Ok(());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Ok(restore) = engine.timeout(now) {
                    finish_restore(&mut engine, &restore)?;
                    write_status(&status_path, &engine, None)?;
                    return Ok(());
                }
            }
        }
    }
}

fn validate_storage_dir(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("watchdog storage path must be absolute".into());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("inspect watchdog storage directory: {error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("watchdog storage path must be a real directory".into());
    }
    path.canonicalize()
        .map_err(|error| format!("canonicalize watchdog storage directory: {error}"))
}

fn run_one_shot_worker(grant: &WorkerGrant) -> Result<bool, String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("resolve fixed actor image: {error}"))?;
    let mut child = Command::new(&executable)
        .arg("--worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("spawn one-shot worker: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "worker stdout unavailable".to_string())?;
    let mut stdout = BufReader::new(stdout);
    let mut hello_line = String::new();
    stdout
        .read_line(&mut hello_line)
        .map_err(|error| format!("read worker HELLO: {error}"))?;
    let hello: WorkerHello = serde_json::from_str(&hello_line)
        .map_err(|error| format!("parse worker HELLO: {error}"))?;
    let expected_image_digest = file_digest(&executable)?;
    #[cfg(target_os = "windows")]
    let expected_creation_time = child_process_creation_time(&child)?;
    #[cfg(not(target_os = "windows"))]
    let expected_creation_time = hello.identity.process_creation_time;
    if hello.schema_version != 1
        || hello.identity.pid != child.id()
        || hello.identity.process_creation_time != expected_creation_time
        || hello.identity.image_digest != expected_image_digest
        || hello.identity.role != WorkerRole::FakeOneShot
        || hello.identity.process_nonce == [0; 16]
    {
        return Err("worker HELLO identity mismatch".into());
    }
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "worker stdin unavailable".to_string())?;
    serde_json::to_writer(
        &mut stdin,
        &WorkerGo {
            schema_version: 1,
            expected_identity: hello.identity.clone(),
            grant: grant.clone(),
        },
    )
    .map_err(|error| format!("write worker GO: {error}"))?;
    stdin
        .flush()
        .map_err(|error| format!("flush worker frame: {error}"))?;
    drop(stdin);
    let mut terminal = String::new();
    stdout
        .read_to_string(&mut terminal)
        .map_err(|error| format!("read worker TERMINAL: {error}"))?;
    let status = child
        .wait()
        .map_err(|error| format!("wait one-shot worker: {error}"))?;
    if !status.success() {
        return Ok(false);
    }
    let result: WorkerResult =
        serde_json::from_str(&terminal).map_err(|error| format!("parse worker result: {error}"))?;
    Ok(result.schema_version == 1
        && result.identity == hello.identity
        && result.operation == grant.operation
        && result.operation_nonce == grant.operation_nonce
        && result.sequence == grant.sequence
        && result.succeeded)
}

#[cfg(target_os = "windows")]
fn current_process_creation_time() -> Result<u64, String> {
    // SAFETY: GetCurrentProcess returns a valid pseudo-handle for this process;
    // all FILETIME outputs point to initialized writable values for the call.
    unsafe { process_creation_time(GetCurrentProcess()) }
}

#[cfg(target_os = "windows")]
fn child_process_creation_time(child: &std::process::Child) -> Result<u64, String> {
    // SAFETY: Child owns a live process handle here and it remains valid for the call.
    unsafe { process_creation_time(HANDLE(child.as_raw_handle())) }
}

#[cfg(target_os = "windows")]
unsafe fn process_creation_time(handle: HANDLE) -> Result<u64, String> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: the handle and output pointers are valid as documented by the callers.
    unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
        .map_err(|error| format!("read process creation time: {error}"))?;
    Ok((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
}

#[cfg(not(target_os = "windows"))]
fn current_process_creation_time() -> Result<u64, String> {
    Ok(current_tick_ms())
}

fn file_digest(path: &Path) -> Result<[u8; 32], String> {
    let bytes = fs::read(path).map_err(|error| format!("read actor image: {error}"))?;
    Ok(Sha256::digest(bytes).into())
}

fn finish_restore(engine: &mut SafetyEngine, grant: &WorkerGrant) -> Result<(), String> {
    let succeeded = run_one_shot_worker(grant)?;
    engine
        .complete_worker(grant, succeeded)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn write_status(path: &Path, engine: &SafetyEngine, error: Option<String>) -> Result<(), String> {
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec(&ActorStatus {
        schema_version: 1,
        status: engine.status(),
        worker_operations_issued: engine.worker_operations_issued(),
        error,
    })
    .map_err(|error| format!("serialize actor status: {error}"))?;
    let mut file =
        fs::File::create(&temporary).map_err(|error| format!("create actor status: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("write actor status: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("flush actor status: {error}"))?;
    drop(file);
    fs::rename(&temporary, path).map_err(|error| format!("publish actor status: {error}"))
}
