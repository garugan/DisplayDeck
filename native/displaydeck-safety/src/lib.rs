#![deny(unsafe_op_in_unsafe_fn)]

mod engine;
mod journal;
mod machine_storage;
mod protocol;
mod provision;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
mod provision_service;
mod wal;

pub use engine::{
    classify_startup, current_tick_ms, digest_label, random_id, ActorFence, BeginInput,
    CommandInput, EngineError, FaultPlan, Operation, SafetyEngine, SafetyStatus, StartupRecovery,
    TerminalDecision, WorkerGrant,
};
pub use journal::{Decision, DecisionJournal, DecisionRecord, JournalClassification};
pub use machine_storage::{
    inspect_machine_actor_storage, D07Anchor, D07StorageFailure, D07StorageVerdict,
};
pub use protocol::{
    ActorStatus, WatchdogCommand, WatchdogStart, WorkerGo, WorkerHello, WorkerIdentity,
    WorkerResult, WorkerRole,
};
pub use provision::{
    classify_candidate04_maprv1, classify_candidate04_provision_pair,
    validate_candidate04_current_provision_link, MachineFileObservation, ProvisionFileIdentity,
    ProvisionPairClassification, ProvisionRecordClassification, ProvisionState,
};
pub use provision_service::{run_system_provision_handshake, run_system_provision_service};
pub use wal::{OperationalWal, WalRecord, WalState};
