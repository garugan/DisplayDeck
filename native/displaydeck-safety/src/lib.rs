#![deny(unsafe_op_in_unsafe_fn)]

mod engine;
mod journal;
mod machine_storage;
mod protocol;
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
pub use wal::{OperationalWal, WalRecord, WalState};
