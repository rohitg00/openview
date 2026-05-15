use openview_core::{CapabilityRisk, FunctionSpec, ResourceKind, WorkerManifest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkerManifestError {
    #[error("manifest JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BinaryWorkerCli {
    pub url: String,
    pub config_path: Option<String>,
    pub manifest_only: bool,
}

impl Default for BinaryWorkerCli {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:49134".to_string(),
            config_path: None,
            manifest_only: false,
        }
    }
}

pub fn runtime_manifest_json(manifest: &WorkerManifest) -> Result<String, WorkerManifestError> {
    Ok(serde_json::to_string_pretty(manifest)?)
}

pub fn approval_worker_manifest() -> WorkerManifest {
    WorkerManifest::new("approval.gate", "0.1.0")
        .description("Human approval queue for risky agent actions")
        .resource(ResourceKind::ApprovalQueue)
        .function(FunctionSpec::new(
            "approval::request",
            CapabilityRisk::Approval,
        ))
        .function(FunctionSpec::new(
            "approval::resolve",
            CapabilityRisk::Approval,
        ))
}

pub fn shell_sandbox_worker_manifest() -> WorkerManifest {
    WorkerManifest::new("shell.sandbox", "0.1.0")
        .description("Sandboxed command execution with explicit policy and approval")
        .resource(ResourceKind::Sandbox)
        .resource(ResourceKind::Process)
        .function(FunctionSpec::new("shell::plan", CapabilityRisk::Read))
        .function(FunctionSpec::new("shell::run", CapabilityRisk::Exec).approval_required(true))
}
