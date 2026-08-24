#![deny(unsafe_op_in_unsafe_fn)]

use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::Mutex,
};

use display_probe::app_snapshot::AppDisplaySnapshot;
use displaydeck_safety::{
    current_tick_ms, digest_label, random_id, ActorFence, ActorStatus, BeginInput, CommandInput,
    FaultPlan, SafetyStatus, WatchdogCommand, WatchdogStart,
};
use serde::{Deserialize, Serialize};
use tauri::WebviewWindow;

struct AppState {
    simulation: Mutex<SimulationManager>,
}

#[derive(Default)]
struct SimulationManager {
    view_revision: Option<[u8; 16]>,
    frontend_boot_nonce: Option<[u8; 16]>,
    session: Option<SimulationSession>,
}

struct SimulationSession {
    transaction_id: String,
    fence: ActorFence,
    deadline_tick_ms: u64,
    status_path: PathBuf,
    presentation_stage: u8,
    child: Child,
    stdin: ChildStdin,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum StatusMode {
    BootHandshake,
    OrdinaryResync,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StatusRequest {
    schema_version: u16,
    mode: StatusMode,
    frontend_boot_nonce: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BeginChangeRequest {
    schema_version: u16,
    view_revision: String,
    simulation: bool,
    duration_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum PresentationStage {
    RevertReady,
    ConfirmationReady,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PresentationAckRequest {
    schema_version: u16,
    view_revision: String,
    transaction_id: String,
    stage: PresentationStage,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DecisionRequest {
    schema_version: u16,
    view_revision: String,
    transaction_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangeStatusResponse {
    schema_version: u16,
    view_revision: String,
    mutation_allowed: bool,
    simulation_allowed: bool,
    transaction_id: Option<String>,
    state: &'static str,
    remaining_ms: Option<u64>,
    presentation_stage: u8,
    message: String,
}

#[tauri::command]
async fn get_display_snapshot(window: WebviewWindow) -> Result<AppDisplaySnapshot, String> {
    ensure_main_window(&window)?;
    tauri::async_runtime::spawn_blocking(display_probe::app_snapshot::capture)
        .await
        .map_err(|error| format!("display snapshot task failed: {error}"))
}

#[tauri::command]
fn get_display_change_status(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    request: StatusRequest,
) -> Result<ChangeStatusResponse, String> {
    ensure_main_window(&window)?;
    validate_schema(request.schema_version)?;
    let frontend_boot_nonce = decode_hex_16(&request.frontend_boot_nonce)?;
    let mut manager = state
        .simulation
        .lock()
        .map_err(|_| "simulation state lock poisoned".to_string())?;
    manager.bind_view(request.mode, frontend_boot_nonce)?;
    manager.project()
}

#[tauri::command]
fn begin_display_change(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    request: BeginChangeRequest,
) -> Result<ChangeStatusResponse, String> {
    ensure_main_window(&window)?;
    validate_schema(request.schema_version)?;
    if !request.simulation {
        return Err(
            "D07 and exact-cell readiness are incomplete; real display changes remain disabled"
                .into(),
        );
    }
    if !(1_000..=15_000).contains(&request.duration_ms) {
        return Err("simulation duration must be between 1000 and 15000 ms".into());
    }
    let mut manager = state
        .simulation
        .lock()
        .map_err(|_| "simulation state lock poisoned".to_string())?;
    manager.validate_view(&request.view_revision)?;
    if let Some(session) = manager.session.as_mut() {
        if !session_is_terminal(session)? {
            return Err("a fake transaction is already active".into());
        }
        let _ = session.child.try_wait();
    }
    manager.session = Some(start_simulation(request.duration_ms)?);
    manager.project()
}

#[tauri::command]
fn ack_display_change_presentation(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    request: PresentationAckRequest,
) -> Result<ChangeStatusResponse, String> {
    ensure_main_window(&window)?;
    validate_schema(request.schema_version)?;
    let mut manager = state
        .simulation
        .lock()
        .map_err(|_| "simulation state lock poisoned".to_string())?;
    manager.validate_view(&request.view_revision)?;
    let session = manager
        .session
        .as_mut()
        .ok_or_else(|| "no fake transaction".to_string())?;
    validate_transaction(session, &request.transaction_id)?;
    let status = read_actor_status(&session.status_path)?;
    if status.status != SafetyStatus::AwaitingDecision {
        return Err("presentation ACK is only valid while awaiting a decision".into());
    }
    session.presentation_stage = match (&request.stage, session.presentation_stage) {
        (PresentationStage::RevertReady, 0) => 1,
        (PresentationStage::RevertReady, 1) => 1,
        (PresentationStage::ConfirmationReady, 1) => 2,
        (PresentationStage::ConfirmationReady, 2) => 2,
        _ => return Err("presentation ACK order mismatch".into()),
    };
    manager.project()
}

#[tauri::command]
fn confirm_display_change(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    request: DecisionRequest,
) -> Result<ChangeStatusResponse, String> {
    decision_command(window, state, request, true)
}

#[tauri::command]
fn revert_display_change(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    request: DecisionRequest,
) -> Result<ChangeStatusResponse, String> {
    decision_command(window, state, request, false)
}

fn decision_command(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    request: DecisionRequest,
    keep: bool,
) -> Result<ChangeStatusResponse, String> {
    ensure_main_window(&window)?;
    validate_schema(request.schema_version)?;
    let mut manager = state
        .simulation
        .lock()
        .map_err(|_| "simulation state lock poisoned".to_string())?;
    manager.validate_view(&request.view_revision)?;
    let session = manager
        .session
        .as_mut()
        .ok_or_else(|| "no fake transaction".to_string())?;
    validate_transaction(session, &request.transaction_id)?;
    if keep && session.presentation_stage != 2 {
        return Err("both presentation stages must be acknowledged before Keep".into());
    }
    let command = CommandInput {
        fence: session.fence.clone(),
        command_nonce: random_id()?,
        now_tick_ms: current_tick_ms(),
    };
    let frame = if keep {
        WatchdogCommand::Confirm { command }
    } else {
        WatchdogCommand::Revert { command }
    };
    write_frame(&mut session.stdin, &frame)?;
    manager.project()
}

impl SimulationManager {
    fn bind_view(&mut self, mode: StatusMode, frontend_boot_nonce: [u8; 16]) -> Result<(), String> {
        match mode {
            StatusMode::BootHandshake if self.frontend_boot_nonce == Some(frontend_boot_nonce) => {}
            StatusMode::BootHandshake => {
                self.frontend_boot_nonce = Some(frontend_boot_nonce);
                self.view_revision = Some(random_id()?);
                if let Some(session) = self.session.as_mut() {
                    session.presentation_stage = 0;
                }
            }
            StatusMode::OrdinaryResync if self.view_revision.is_none() => {
                return Err("BOOT_HANDSHAKE is required before ordinary resync".into());
            }
            StatusMode::OrdinaryResync if self.frontend_boot_nonce != Some(frontend_boot_nonce) => {
                return Err("stale frontend boot nonce".into());
            }
            StatusMode::OrdinaryResync => {}
        }
        Ok(())
    }

    fn validate_view(&self, provided: &str) -> Result<(), String> {
        let expected = self
            .view_revision
            .ok_or_else(|| "BOOT_HANDSHAKE is required".to_string())?;
        if decode_hex_16(provided)? != expected {
            return Err("stale view revision".into());
        }
        Ok(())
    }

    fn project(&mut self) -> Result<ChangeStatusResponse, String> {
        let view_revision = encode_hex(
            &self
                .view_revision
                .ok_or_else(|| "BOOT_HANDSHAKE is required".to_string())?,
        );
        let Some(session) = self.session.as_mut() else {
            return Ok(ChangeStatusResponse {
                schema_version: 1,
                view_revision,
                mutation_allowed: false,
                simulation_allowed: true,
                transaction_id: None,
                state: "IDLE",
                remaining_ms: None,
                presentation_stage: 0,
                message: "Windowsの設定は変更されていません".into(),
            });
        };
        let child_exited = session
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some();
        let (state, message) = match read_actor_status(&session.status_path) {
            Ok(status) if child_exited && !matches!(status.status, SafetyStatus::Terminal(_)) => (
                "FAILED_CLOSED",
                "fake watchdogがterminal状態を残さず終了しました".into(),
            ),
            Ok(status) => project_actor_status(&status),
            Err(error) if child_exited => (
                "FAILED_CLOSED",
                format!("fake watchdogの終了状態を確認できません: {error}"),
            ),
            Err(_) => ("STARTING", "fake watchdogを起動しています".into()),
        };
        let remaining_ms = matches!(state, "STARTING" | "APPLY_IN_FLIGHT" | "AWAITING_DECISION")
            .then(|| session.deadline_tick_ms.saturating_sub(current_tick_ms()));
        Ok(ChangeStatusResponse {
            schema_version: 1,
            view_revision,
            mutation_allowed: false,
            simulation_allowed: true,
            transaction_id: Some(session.transaction_id.clone()),
            state,
            remaining_ms,
            presentation_stage: session.presentation_stage,
            message,
        })
    }
}

fn start_simulation(duration_ms: u64) -> Result<SimulationSession, String> {
    let session_id = random_id()?;
    let transaction_id = encode_hex(&session_id);
    let root = std::env::temp_dir().join("DisplayDeck-Stage1");
    fs::create_dir_all(&root).map_err(|error| format!("create Stage 1 test root: {error}"))?;
    let storage_dir = root.join(&transaction_id);
    fs::create_dir(&storage_dir)
        .map_err(|error| format!("create Stage 1 transaction directory: {error}"))?;

    let controller_instance_id = random_id()?;
    let watchdog_instance_id = random_id()?;
    let boot_seed = random_id()?;
    let fence = ActorFence {
        session_id,
        boot_id: digest_label(&boot_seed),
        controller_instance_id,
        watchdog_instance_id,
        display_id: digest_label(b"DisplayDeck.Stage1.FakeDisplay"),
        owner_sid_digest: digest_label(b"DisplayDeck.Stage1.FakeOwner"),
        logon_id: 1,
        lease_version: 1,
    };
    let now_tick_ms = current_tick_ms();
    let deadline_tick_ms = now_tick_ms
        .checked_add(duration_ms)
        .ok_or_else(|| "simulation deadline overflow".to_string())?;
    let start = WatchdogStart {
        schema_version: 1,
        storage_dir: storage_dir.clone(),
        input: BeginInput {
            fence: fence.clone(),
            machine_epoch: u64::from_le_bytes(session_id[..8].try_into().unwrap()).max(1),
            now_tick_ms,
            confirmation_deadline_tick_ms: deadline_tick_ms,
            previous_display_mode_digest: digest_label(b"DisplayDeck.Stage1.C0"),
            expected_rollback_snapshot_digest: digest_label(b"DisplayDeck.Stage1.Rollback"),
            candidate_digest: digest_label(b"DisplayDeck.Stage1.FakeCandidate"),
            expected_display_mode_digest: digest_label(b"DisplayDeck.Stage1.FakeExpected"),
        },
        fault_plan: FaultPlan::None,
    };

    let mut child = Command::new(actor_path()?)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("start fixed fake watchdog: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "fake watchdog stdin unavailable".to_string())?;
    write_frame(&mut stdin, &start)?;

    Ok(SimulationSession {
        transaction_id,
        fence,
        deadline_tick_ms,
        status_path: storage_dir.join("status.json"),
        presentation_stage: 0,
        child,
        stdin,
    })
}

fn actor_path() -> Result<PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("resolve DisplayDeck executable: {error}"))?;
    let name = if cfg!(target_os = "windows") {
        "displaydeck-actor.exe"
    } else {
        "displaydeck-actor"
    };
    let path = executable.with_file_name(name);
    path.is_file().then_some(path).ok_or_else(|| {
        "fixed displaydeck-actor sibling is missing; build the workspace first".into()
    })
}

fn read_actor_status(path: &PathBuf) -> Result<ActorStatus, String> {
    let bytes = fs::read(path).map_err(|error| format!("read fake watchdog status: {error}"))?;
    let status: ActorStatus = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse fake watchdog status: {error}"))?;
    if status.schema_version != 1 {
        return Err("unsupported fake watchdog status schema".into());
    }
    Ok(status)
}

fn session_is_terminal(session: &mut SimulationSession) -> Result<bool, String> {
    match read_actor_status(&session.status_path) {
        Ok(status) => Ok(matches!(status.status, SafetyStatus::Terminal(_))),
        Err(_) => Ok(session
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()),
    }
}

fn validate_transaction(session: &SimulationSession, provided: &str) -> Result<(), String> {
    if provided != session.transaction_id {
        return Err("stale transaction identity".into());
    }
    Ok(())
}

fn project_actor_status(status: &ActorStatus) -> (&'static str, String) {
    use displaydeck_safety::TerminalDecision;
    match status.status {
        SafetyStatus::ApplyInFlight => ("APPLY_IN_FLIGHT", "fake workerを実行しています".into()),
        SafetyStatus::AwaitingDecision => (
            "AWAITING_DECISION",
            "維持するか戻すかを選んでください".into(),
        ),
        SafetyStatus::KeepAuthorized => ("KEEP_AUTHORIZED", "確定処理中です".into()),
        SafetyStatus::RevertInFlight => ("REVERT_IN_FLIGHT", "fake baselineへ戻しています".into()),
        SafetyStatus::Terminal(TerminalDecision::KeptSession) => {
            ("KEPT_SESSION", "fake transactionを維持しました".into())
        }
        SafetyStatus::Terminal(TerminalDecision::Reverted) => {
            ("REVERTED", "fake baselineへ戻しました".into())
        }
        SafetyStatus::Terminal(TerminalDecision::FailedClosed) => (
            "FAILED_CLOSED",
            status
                .error
                .clone()
                .unwrap_or_else(|| "安全側で停止しました".into()),
        ),
    }
}

fn write_frame(writer: &mut ChildStdin, value: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, value)
        .map_err(|error| format!("serialize watchdog frame: {error}"))?;
    writer
        .write_all(b"\n")
        .map_err(|error| format!("write watchdog frame: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("flush watchdog frame: {error}"))
}

fn ensure_main_window(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("command is restricted to the local main window".into());
    }
    Ok(())
}

fn validate_schema(version: u16) -> Result<(), String> {
    if version != 1 {
        return Err("unsupported command schema".into());
    }
    Ok(())
}

fn decode_hex_16(value: &str) -> Result<[u8; 16], String> {
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("token must be exactly 32 hexadecimal characters".into());
    }
    let mut output = [0_u8; 16];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "invalid hexadecimal token".to_string())?;
    }
    if output == [0; 16] {
        return Err("all-zero token is invalid".into());
    }
    Ok(output)
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .manage(AppState {
            simulation: Mutex::new(SimulationManager::default()),
        })
        .invoke_handler(tauri::generate_handler![
            get_display_snapshot,
            begin_display_change,
            ack_display_change_presentation,
            confirm_display_change,
            revert_display_change,
            get_display_change_status
        ])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_handshake_is_idempotent_and_resync_is_bound() {
        let mut manager = SimulationManager::default();
        manager
            .bind_view(StatusMode::BootHandshake, [1; 16])
            .unwrap();
        let first = manager.view_revision;
        manager
            .bind_view(StatusMode::BootHandshake, [1; 16])
            .unwrap();
        assert_eq!(manager.view_revision, first);
        assert!(manager
            .bind_view(StatusMode::OrdinaryResync, [2; 16])
            .is_err());
    }
}
