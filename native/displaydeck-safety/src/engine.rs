use std::{fmt, path::Path};

#[cfg(not(target_os = "windows"))]
use std::{sync::OnceLock, time::Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    DecisionJournal, DecisionRecord, JournalClassification, OperationalWal, WalRecord, WalState,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorFence {
    pub session_id: [u8; 16],
    pub boot_id: [u8; 32],
    pub controller_instance_id: [u8; 16],
    pub watchdog_instance_id: [u8; 16],
    pub display_id: [u8; 32],
    pub owner_sid_digest: [u8; 32],
    pub logon_id: u64,
    pub lease_version: u64,
}

impl ActorFence {
    pub fn is_valid(&self) -> bool {
        self.session_id != [0; 16]
            && self.boot_id != [0; 32]
            && self.controller_instance_id != [0; 16]
            && self.watchdog_instance_id != [0; 16]
            && self.display_id != [0; 32]
            && self.owner_sid_digest != [0; 32]
            && self.logon_id != 0
            && self.lease_version != 0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginInput {
    pub fence: ActorFence,
    pub machine_epoch: u64,
    pub now_tick_ms: u64,
    pub confirmation_deadline_tick_ms: u64,
    pub previous_display_mode_digest: [u8; 32],
    pub expected_rollback_snapshot_digest: [u8; 32],
    pub candidate_digest: [u8; 32],
    pub expected_display_mode_digest: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandInput {
    pub fence: ActorFence,
    pub command_nonce: [u8; 16],
    pub now_tick_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaultPlan {
    #[default]
    None,
    BaselineReadbackFailure,
    KeepPublicationFailure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Operation {
    FakeApply,
    FakeRestore,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerGrant {
    pub fence: ActorFence,
    pub machine_epoch: u64,
    pub operation: Operation,
    pub operation_nonce: [u8; 16],
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TerminalDecision {
    KeptSession,
    Reverted,
    FailedClosed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StartupRecovery {
    NoTransaction,
    Keep,
    Revert,
    FailedClosed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SafetyStatus {
    ApplyInFlight,
    AwaitingDecision,
    KeepAuthorized,
    RevertInFlight,
    Terminal(TerminalDecision),
}

#[derive(Debug, Eq, PartialEq)]
pub enum EngineError {
    InvalidAuthority,
    DeadlineExpired,
    DuplicateCommand,
    StaleWorker,
    WrongState,
    Storage(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAuthority => write!(formatter, "invalid or stale authority"),
            Self::DeadlineExpired => write!(formatter, "confirmation deadline expired"),
            Self::DuplicateCommand => write!(formatter, "duplicate command rejected"),
            Self::StaleWorker => write!(formatter, "stale or duplicate worker rejected"),
            Self::WrongState => write!(formatter, "command is invalid in the current state"),
            Self::Storage(error) => write!(formatter, "durable state failure: {error}"),
        }
    }
}

pub struct SafetyEngine {
    fence: ActorFence,
    machine_epoch: u64,
    deadline_tick_ms: u64,
    candidate_digest: [u8; 32],
    expected_display_mode_digest: [u8; 32],
    journal: DecisionJournal,
    wal: OperationalWal,
    wal_generation: u64,
    status: SafetyStatus,
    outstanding_worker: Option<WorkerGrant>,
    next_worker_sequence: u64,
    seen_commands: Vec<[u8; 16]>,
    worker_operations_issued: usize,
    fault_plan: FaultPlan,
}

impl SafetyEngine {
    pub fn begin(
        storage_dir: &Path,
        input: BeginInput,
        fault_plan: FaultPlan,
    ) -> Result<(Self, WorkerGrant), EngineError> {
        if !input.fence.is_valid()
            || input.machine_epoch == 0
            || input.previous_display_mode_digest == [0; 32]
            || input.expected_rollback_snapshot_digest == [0; 32]
            || input.candidate_digest == [0; 32]
            || input.expected_display_mode_digest == [0; 32]
        {
            return Err(EngineError::InvalidAuthority);
        }
        if input.now_tick_ms > input.confirmation_deadline_tick_ms {
            return Err(EngineError::DeadlineExpired);
        }

        let journal = DecisionJournal::create_test_storage(storage_dir.join("DecisionJournalV1"))
            .map_err(EngineError::Storage)?;
        let wal = OperationalWal::create_test_storage(storage_dir.join("OperationalWalV1"))
            .map_err(EngineError::Storage)?;
        let baseline = DecisionRecord::baseline(
            input.fence.clone(),
            input.now_tick_ms,
            input.previous_display_mode_digest,
            input.expected_rollback_snapshot_digest,
        );
        let readback = journal.publish(&baseline).map_err(EngineError::Storage)?;
        if fault_plan == FaultPlan::BaselineReadbackFailure
            || !matches!(readback, JournalClassification::Baseline(_))
        {
            return Err(EngineError::Storage(
                "baseline close/reopen readback failed".into(),
            ));
        }

        let mut engine = Self {
            fence: input.fence,
            machine_epoch: input.machine_epoch,
            deadline_tick_ms: input.confirmation_deadline_tick_ms,
            candidate_digest: input.candidate_digest,
            expected_display_mode_digest: input.expected_display_mode_digest,
            journal,
            wal,
            wal_generation: 0,
            status: SafetyStatus::ApplyInFlight,
            outstanding_worker: None,
            next_worker_sequence: 1,
            seen_commands: Vec::new(),
            worker_operations_issued: 0,
            fault_plan,
        };
        engine.publish_wal(WalState::BaselineProvisioned)?;
        let grant = engine.issue_worker(Operation::FakeApply, WalState::ApplyInFlight)?;
        Ok((engine, grant))
    }

    pub fn status(&self) -> SafetyStatus {
        self.status
    }

    pub fn worker_operations_issued(&self) -> usize {
        self.worker_operations_issued
    }

    pub fn complete_worker(
        &mut self,
        grant: &WorkerGrant,
        succeeded: bool,
    ) -> Result<Option<WorkerGrant>, EngineError> {
        if self.outstanding_worker.as_ref() != Some(grant) || grant.fence != self.fence {
            return Err(EngineError::StaleWorker);
        }
        self.outstanding_worker = None;
        match (grant.operation, succeeded) {
            (Operation::FakeApply, true) => {
                self.status = SafetyStatus::AwaitingDecision;
                self.publish_wal(WalState::AwaitingDecision)?;
                Ok(None)
            }
            (Operation::FakeApply, false) => {
                let restore =
                    self.issue_worker(Operation::FakeRestore, WalState::RevertInFlight)?;
                Ok(Some(restore))
            }
            (Operation::FakeRestore, true) => {
                self.status = SafetyStatus::Terminal(TerminalDecision::Reverted);
                self.publish_wal(WalState::Reverted)?;
                Ok(None)
            }
            (Operation::FakeRestore, false) => {
                self.status = SafetyStatus::Terminal(TerminalDecision::FailedClosed);
                self.publish_wal(WalState::FailedClosed)?;
                Err(EngineError::Storage("fake restore failed".into()))
            }
        }
    }

    pub fn confirm(&mut self, command: CommandInput) -> Result<SafetyStatus, EngineError> {
        self.validate_command(&command)?;
        if self.status != SafetyStatus::AwaitingDecision {
            return Err(EngineError::WrongState);
        }
        if command.now_tick_ms > self.deadline_tick_ms {
            return Err(EngineError::DeadlineExpired);
        }
        self.status = SafetyStatus::KeepAuthorized;
        self.publish_wal(WalState::KeepAuthorized)?;

        if self.fault_plan == FaultPlan::KeepPublicationFailure {
            return Err(self.fail_closed("Keep publication failed"));
        }
        let kept = DecisionRecord::kept(
            self.fence.clone(),
            self.deadline_tick_ms,
            command.now_tick_ms,
            current_tick_ms().max(command.now_tick_ms),
            self.candidate_digest,
            self.expected_display_mode_digest,
        );
        let publication = self.journal.publish(&kept);
        if !matches!(publication, Ok(JournalClassification::Kept { .. })) {
            let reason = publication
                .err()
                .unwrap_or_else(|| "Keep readback classification mismatch".into());
            return Err(self.fail_closed(&reason));
        }
        self.status = SafetyStatus::Terminal(TerminalDecision::KeptSession);
        self.publish_wal(WalState::KeptSession)?;
        Ok(self.status)
    }

    pub fn manual_revert(&mut self, command: CommandInput) -> Result<WorkerGrant, EngineError> {
        self.validate_command(&command)?;
        self.start_revert()
    }

    pub fn timeout(&mut self, now_tick_ms: u64) -> Result<WorkerGrant, EngineError> {
        if now_tick_ms < self.deadline_tick_ms {
            return Err(EngineError::WrongState);
        }
        self.start_revert()
    }

    pub fn parent_loss(&mut self) -> Result<WorkerGrant, EngineError> {
        self.start_revert()
    }

    fn start_revert(&mut self) -> Result<WorkerGrant, EngineError> {
        if self.status != SafetyStatus::AwaitingDecision {
            return Err(EngineError::WrongState);
        }
        self.issue_worker(Operation::FakeRestore, WalState::RevertInFlight)
    }

    fn validate_command(&mut self, command: &CommandInput) -> Result<(), EngineError> {
        if command.fence != self.fence || command.command_nonce == [0; 16] {
            return Err(EngineError::InvalidAuthority);
        }
        if self.seen_commands.contains(&command.command_nonce) {
            return Err(EngineError::DuplicateCommand);
        }
        self.seen_commands.push(command.command_nonce);
        Ok(())
    }

    fn issue_worker(
        &mut self,
        operation: Operation,
        wal_state: WalState,
    ) -> Result<WorkerGrant, EngineError> {
        if self.outstanding_worker.is_some() {
            return Err(EngineError::WrongState);
        }
        let grant = WorkerGrant {
            fence: self.fence.clone(),
            machine_epoch: self.machine_epoch,
            operation,
            operation_nonce: random_id().map_err(EngineError::Storage)?,
            sequence: self.next_worker_sequence,
        };
        self.next_worker_sequence = self
            .next_worker_sequence
            .checked_add(1)
            .ok_or_else(|| EngineError::Storage("worker sequence overflow".into()))?;
        self.worker_operations_issued += 1;
        self.status = match operation {
            Operation::FakeApply => SafetyStatus::ApplyInFlight,
            Operation::FakeRestore => SafetyStatus::RevertInFlight,
        };
        self.outstanding_worker = Some(grant.clone());
        self.publish_wal(wal_state)?;
        Ok(grant)
    }

    fn publish_wal(&mut self, state: WalState) -> Result<(), EngineError> {
        let generation = self
            .wal_generation
            .checked_add(1)
            .ok_or_else(|| EngineError::Storage("WAL generation overflow".into()))?;
        self.wal
            .publish(&WalRecord {
                generation,
                previous_generation: self.wal_generation,
                fence: self.fence.clone(),
                machine_epoch: self.machine_epoch,
                state,
            })
            .map_err(EngineError::Storage)?;
        self.wal_generation = generation;
        Ok(())
    }

    fn fail_closed(&mut self, reason: &str) -> EngineError {
        self.status = SafetyStatus::Terminal(TerminalDecision::FailedClosed);
        let _ = self.publish_wal(WalState::FailedClosed);
        EngineError::Storage(reason.into())
    }
}

pub fn random_id() -> Result<[u8; 16], String> {
    let mut value = [0_u8; 16];
    getrandom::fill(&mut value).map_err(|error| format!("CSPRNG failure: {error}"))?;
    if value == [0; 16] {
        return Err("CSPRNG returned an all-zero identifier".into());
    }
    Ok(value)
}

pub fn digest_label(label: &[u8]) -> [u8; 32] {
    Sha256::digest(label).into()
}

pub fn classify_startup(storage_dir: &Path, fence: &ActorFence) -> StartupRecovery {
    let journal_path = storage_dir.join("DecisionJournalV1");
    let wal_path = storage_dir.join("OperationalWalV1");
    if !journal_path.exists() && !wal_path.exists() {
        return StartupRecovery::NoTransaction;
    }
    if !journal_path.is_file() || !wal_path.is_file() {
        return StartupRecovery::FailedClosed;
    }
    match DecisionJournal::open_test_storage(journal_path).classify_for(fence) {
        Ok(JournalClassification::Kept { .. }) => StartupRecovery::Keep,
        Ok(JournalClassification::Baseline(_)) => {
            match OperationalWal::open_test_storage(wal_path).read_latest_for(fence) {
                Ok(Some(_)) => StartupRecovery::Revert,
                _ => StartupRecovery::FailedClosed,
            }
        }
        _ => StartupRecovery::FailedClosed,
    }
}

#[cfg(target_os = "windows")]
pub fn current_tick_ms() -> u64 {
    // SAFETY: GetTickCount64 takes no pointers or handles and has no preconditions.
    unsafe { windows::Win32::System::SystemInformation::GetTickCount64() }
}

#[cfg(not(target_os = "windows"))]
pub fn current_tick_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    fn fence() -> ActorFence {
        ActorFence {
            session_id: [1; 16],
            boot_id: [2; 32],
            controller_instance_id: [3; 16],
            watchdog_instance_id: [4; 16],
            display_id: [5; 32],
            owner_sid_digest: [6; 32],
            logon_id: 7,
            lease_version: 1,
        }
    }

    fn input() -> BeginInput {
        BeginInput {
            fence: fence(),
            machine_epoch: 1,
            now_tick_ms: 100,
            confirmation_deadline_tick_ms: 15_100,
            previous_display_mode_digest: [7; 32],
            expected_rollback_snapshot_digest: [8; 32],
            candidate_digest: [9; 32],
            expected_display_mode_digest: [10; 32],
        }
    }

    fn command(nonce: u8, now: u64) -> CommandInput {
        CommandInput {
            fence: fence(),
            command_nonce: [nonce; 16],
            now_tick_ms: now,
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "displaydeck-{label}-{}-{}",
            std::process::id(),
            random_id().unwrap()[0]
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    fn awaiting_engine(label: &str) -> (PathBuf, SafetyEngine, WorkerGrant) {
        let path = temp_dir(label);
        let (mut engine, grant) = SafetyEngine::begin(&path, input(), FaultPlan::None).unwrap();
        engine.complete_worker(&grant, true).unwrap();
        (path, engine, grant)
    }

    #[test]
    fn contract_1_baseline_readback_failure_issues_no_worker() {
        let path = temp_dir("baseline-fail");
        let result = SafetyEngine::begin(&path, input(), FaultPlan::BaselineReadbackFailure);
        assert!(matches!(result, Err(EngineError::Storage(_))));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn contract_2_only_valid_keep_becomes_terminal_keep() {
        let (path, mut engine, _) = awaiting_engine("keep");
        assert_eq!(
            engine.confirm(command(11, 1_000)).unwrap(),
            SafetyStatus::Terminal(TerminalDecision::KeptSession)
        );
        let path2 = temp_dir("keep-fail");
        let (mut failed, grant) =
            SafetyEngine::begin(&path2, input(), FaultPlan::KeepPublicationFailure).unwrap();
        failed.complete_worker(&grant, true).unwrap();
        assert!(failed.confirm(command(12, 1_000)).is_err());
        assert_eq!(
            failed.status(),
            SafetyStatus::Terminal(TerminalDecision::FailedClosed)
        );
        fs::remove_dir_all(path).unwrap();
        fs::remove_dir_all(path2).unwrap();
    }

    #[test]
    fn contract_3_revert_timeout_and_parent_loss_restore() {
        for trigger in 0..3 {
            let (path, mut engine, _) = awaiting_engine(&format!("revert-{trigger}"));
            let grant = match trigger {
                0 => engine.manual_revert(command(20, 500)).unwrap(),
                1 => engine.timeout(15_100).unwrap(),
                _ => engine.parent_loss().unwrap(),
            };
            assert_eq!(grant.operation, Operation::FakeRestore);
            engine.complete_worker(&grant, true).unwrap();
            assert_eq!(
                engine.status(),
                SafetyStatus::Terminal(TerminalDecision::Reverted)
            );
            fs::remove_dir_all(path).unwrap();
        }
    }

    #[test]
    fn contract_4_foreign_or_invalid_journal_fails_closed() {
        let (path, engine, _) = awaiting_engine("foreign");
        let mut foreign = fence();
        foreign.boot_id[0] ^= 1;
        assert!(matches!(
            engine.journal.classify_for(&foreign).unwrap(),
            JournalClassification::FailedClosed(_)
        ));
        assert_eq!(
            classify_startup(&path, &foreign),
            StartupRecovery::FailedClosed
        );
        assert_eq!(classify_startup(&path, &fence()), StartupRecovery::Revert);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn contract_5_old_worker_and_duplicate_command_are_rejected() {
        let path = temp_dir("worker-fence");
        let (mut engine, grant) = SafetyEngine::begin(&path, input(), FaultPlan::None).unwrap();
        let mut stale = grant.clone();
        stale.fence.lease_version += 1;
        assert_eq!(
            engine.complete_worker(&stale, true),
            Err(EngineError::StaleWorker)
        );
        engine.complete_worker(&grant, true).unwrap();
        let duplicate = command(30, 500);
        let restore = engine.manual_revert(duplicate.clone()).unwrap();
        assert_eq!(
            engine.manual_revert(duplicate),
            Err(EngineError::DuplicateCommand)
        );
        engine.complete_worker(&restore, true).unwrap();
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn contract_6_deadline_and_identity_mismatch_issue_no_authority() {
        let path = temp_dir("deadline");
        let mut expired = input();
        expired.now_tick_ms = expired.confirmation_deadline_tick_ms + 1;
        assert!(matches!(
            SafetyEngine::begin(&path, expired, FaultPlan::None),
            Err(EngineError::DeadlineExpired)
        ));
        fs::remove_dir_all(path).unwrap();

        let (path, mut engine, _) = awaiting_engine("identity");
        let mut wrong = command(40, 500);
        wrong.fence.display_id[0] ^= 1;
        assert_eq!(engine.confirm(wrong), Err(EngineError::InvalidAuthority));
        assert_eq!(engine.status(), SafetyStatus::AwaitingDecision);
        fs::remove_dir_all(path).unwrap();
    }
}
