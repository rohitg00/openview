use openview_core::{CapabilityRisk, FunctionSpec, SandboxProfile, WorkerManifest};
use openview_worker::{CredentialReference, WorkerExecutionDecisionKind, WorkerExecutionRequest};
use serde_json::json;

fn manifest() -> WorkerManifest {
    WorkerManifest::new("shell.sandbox", "0.1.0")
        .function(FunctionSpec::new("shell::run", CapabilityRisk::Exec).approval_required(true))
        .function(FunctionSpec::new("shell::plan", CapabilityRisk::Read))
        .sandbox(
            SandboxProfile::locked_down()
                .allow_workspace_read("/workspace")
                .allow_workspace_write("/workspace/out")
                .allow_command("python")
                .allow_network_host("api.example.com")
                .allow_secret_name("OPENAI_API_KEY"),
        )
}

#[test]
fn denied_paths_block_before_execution() {
    let decision = WorkerExecutionRequest::new("shell.sandbox", "shell::plan")
        .read_path("/workspace/private/../../etc/passwd")
        .write_path("/workspace/src/lib.rs")
        .evaluate_against(&manifest(), &[]);

    assert_eq!(decision.kind, WorkerExecutionDecisionKind::Blocked);
    assert_eq!(
        decision.reasons,
        vec![
            "read path denied: /workspace/private/../../etc/passwd",
            "write path denied: /workspace/src/lib.rs",
        ]
    );
}

#[test]
fn disallowed_commands_block_before_execution() {
    let decision = WorkerExecutionRequest::new("shell.sandbox", "shell::plan")
        .command("bash")
        .evaluate_against(&manifest(), &[]);

    assert_eq!(decision.kind, WorkerExecutionDecisionKind::Blocked);
    assert_eq!(decision.reasons, vec!["command denied: bash"]);
}

#[test]
fn denied_hosts_block_before_execution() {
    let decision = WorkerExecutionRequest::new("shell.sandbox", "shell::plan")
        .network_host("metadata.google.internal")
        .evaluate_against(&manifest(), &[]);

    assert_eq!(decision.kind, WorkerExecutionDecisionKind::Blocked);
    assert_eq!(
        decision.reasons,
        vec!["network host denied: metadata.google.internal"]
    );
}

#[test]
fn unresolved_approval_required_function_requests_approval() {
    let decision = WorkerExecutionRequest::new("shell.sandbox", "shell::run")
        .command("python")
        .read_path("/workspace/app.py")
        .write_path("/workspace/out/result.json")
        .network_host("api.example.com")
        .credential(CredentialReference::named("OPENAI_API_KEY"))
        .evaluate_against(&manifest(), &[]);

    assert_eq!(decision.kind, WorkerExecutionDecisionKind::ApprovalRequired);
    assert_eq!(decision.reasons, vec!["approval required: shell::run"]);
}

#[test]
fn allowed_safe_requests_return_deterministic_allow_decision() {
    let decision = WorkerExecutionRequest::new("shell.sandbox", "shell::run")
        .command("python")
        .arg("script.py")
        .read_path("/workspace/app.py")
        .write_path("/workspace/out/result.json")
        .network_host("API.EXAMPLE.COM.")
        .credential(CredentialReference::named("OPENAI_API_KEY"))
        .payload(json!({"task":"summarize"}))
        .evaluate_against(&manifest(), &["shell::run"]);

    assert_eq!(decision.kind, WorkerExecutionDecisionKind::Allowed);
    assert!(decision.reasons.is_empty());
    assert_eq!(decision.payload_summary["worker_id"], "shell.sandbox");
    assert_eq!(decision.payload_summary["function_id"], "shell::run");
    assert_eq!(decision.payload_summary["command"], "python");
    assert_eq!(decision.payload_summary["argv"], json!(["script.py"]));
    assert_eq!(
        decision.payload_summary["read_paths"],
        json!(["/workspace/app.py"])
    );
    assert_eq!(
        decision.payload_summary["write_paths"],
        json!(["/workspace/out/result.json"])
    );
    assert_eq!(
        decision.payload_summary["network_hosts"],
        json!(["API.EXAMPLE.COM."])
    );
    assert_eq!(
        decision.payload_summary["credential_refs"],
        json!(["OPENAI_API_KEY"])
    );
}

#[test]
fn payload_summary_redacts_secret_values_and_denies_unlisted_secret_refs() {
    let decision = WorkerExecutionRequest::new("shell.sandbox", "shell::plan")
        .credential(CredentialReference::with_value(
            "OPENAI_API_KEY",
            "sk-test-123",
        ))
        .credential(CredentialReference::with_value(
            "UNLISTED_TOKEN",
            "super-secret-token",
        ))
        .payload(json!({
            "api_key": "sk-test-123",
            "nested": {"password": "super-secret-token"},
            "safe": "visible"
        }))
        .evaluate_against(&manifest(), &[]);

    assert_eq!(decision.kind, WorkerExecutionDecisionKind::Blocked);
    assert_eq!(
        decision.reasons,
        vec!["credential reference denied: UNLISTED_TOKEN"]
    );

    let summary = serde_json::to_string(&decision.payload_summary).unwrap();
    assert!(!summary.contains("sk-test-123"));
    assert!(!summary.contains("super-secret-token"));
    assert!(summary.contains("[redacted]"));
    assert!(summary.contains("OPENAI_API_KEY"));
    assert!(summary.contains("UNLISTED_TOKEN"));
    assert!(summary.contains("visible"));
}
