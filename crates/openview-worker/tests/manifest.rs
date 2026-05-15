use openview_core::{CapabilityRisk, ResourceKind};
use openview_worker::{
    claude_code_agent_worker_manifest, codex_agent_worker_manifest, git_worktree_worker_manifest,
    hermes_agent_worker_manifest, opencode_agent_worker_manifest, runtime_manifest_json,
    terminal_pty_worker_manifest,
};

#[test]
fn worker_crate_exposes_terminal_pty_manifest_for_cli_use() {
    let manifest = terminal_pty_worker_manifest();
    let json = runtime_manifest_json(&manifest).expect("manifest serializes");

    assert_eq!(manifest.name, "terminal.pty");
    assert!(json.contains("terminal.pty::spawn"));
    assert!(json.contains("terminal.pty::read"));
    assert!(json.contains("terminal.pty::resize"));
}

#[test]
fn worker_crate_exposes_git_worktree_manifest_for_cli_use() {
    let manifest = git_worktree_worker_manifest();
    let json = runtime_manifest_json(&manifest).expect("manifest serializes");

    assert_eq!(manifest.name, "git.worktree");
    assert!(json.contains("git.worktree::list"));
    assert!(json.contains("git.worktree::create"));
}

#[test]
fn worker_crate_exposes_hermes_agent_manifest_contract() {
    let manifest = hermes_agent_worker_manifest();
    let json = runtime_manifest_json(&manifest).expect("manifest serializes");

    assert_eq!(manifest.name, "hermes.agent");
    assert_eq!(manifest.binary_name.as_deref(), Some("hermes"));
    assert!(manifest.resources.contains(&ResourceKind::SessionState));
    assert!(manifest.resources.contains(&ResourceKind::EventStream));
    assert!(manifest.resources.contains(&ResourceKind::ApprovalQueue));
    assert!(manifest.resources.contains(&ResourceKind::Filesystem));
    assert!(manifest.resources.contains(&ResourceKind::Process));

    let spawn = manifest
        .functions
        .iter()
        .find(|function| function.id == "hermes.agent::spawn_session")
        .expect("spawn_session function is declared");
    assert_eq!(spawn.risk, CapabilityRisk::Exec);
    assert!(spawn.approval_required);
    assert_eq!(
        spawn.request_schema["required"],
        serde_json::json!(["profile", "prompt", "workspace"])
    );
    assert!(spawn.request_schema["properties"]
        .get("session_id")
        .is_some());
    assert!(spawn.response_schema["properties"]
        .get("session_id")
        .is_some());
    assert!(spawn.response_schema["properties"]
        .get("event_stream")
        .is_some());

    assert!(json.contains("hermes.agent::send_prompt"));
    assert!(json.contains("hermes.agent::stream_events"));
    assert!(json.contains("hermes.agent::resolve_approval"));
    assert!(json.contains("HERMES_PROFILE"));
}

#[test]
fn worker_crate_exposes_codex_claude_and_opencode_agent_manifests() {
    for (manifest, function_prefix, binary) in [
        (codex_agent_worker_manifest(), "codex.agent", "codex"),
        (
            claude_code_agent_worker_manifest(),
            "claude-code.agent",
            "claude",
        ),
        (
            opencode_agent_worker_manifest(),
            "opencode.agent",
            "opencode",
        ),
    ] {
        let json = runtime_manifest_json(&manifest).expect("manifest serializes");
        assert_eq!(manifest.name, function_prefix);
        assert_eq!(manifest.binary_name.as_deref(), Some(binary));
        assert!(manifest.resources.contains(&ResourceKind::GitWorktree));
        assert!(manifest.resources.contains(&ResourceKind::Process));
        assert!(manifest.resources.contains(&ResourceKind::SessionState));
        assert!(manifest.resources.contains(&ResourceKind::EventStream));
        assert!(manifest.dependencies.contains("git.worktree"));
        assert!(manifest.dependencies.contains("shell.sandbox"));
        assert!(manifest.dependencies.contains("approval.gate"));
        assert!(json.contains(&format!("{function_prefix}::spawn_session")));
        assert!(json.contains(&format!("{function_prefix}::stream_events")));
    }
}
