use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{BeginInput, CommandInput, FaultPlan, Operation, SafetyStatus, WorkerGrant};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchdogStart {
    pub schema_version: u16,
    pub storage_dir: PathBuf,
    pub input: BeginInput,
    #[serde(default)]
    pub fault_plan: FaultPlan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum WatchdogCommand {
    Confirm { command: CommandInput },
    Revert { command: CommandInput },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorStatus {
    pub schema_version: u16,
    pub status: SafetyStatus,
    pub worker_operations_issued: usize,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkerRole {
    FakeOneShot,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerIdentity {
    pub pid: u32,
    pub process_creation_time: u64,
    pub image_digest: [u8; 32],
    pub role: WorkerRole,
    pub process_nonce: [u8; 16],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerHello {
    pub schema_version: u16,
    pub identity: WorkerIdentity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerGo {
    pub schema_version: u16,
    pub expected_identity: WorkerIdentity,
    pub grant: WorkerGrant,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerResult {
    pub schema_version: u16,
    pub identity: WorkerIdentity,
    pub operation: Operation,
    pub operation_nonce: [u8; 16],
    pub sequence: u64,
    pub succeeded: bool,
}
