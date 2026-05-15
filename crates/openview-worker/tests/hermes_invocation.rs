use openview_worker::{HermesApprovalPolicy, HermesSessionRequest};

#[test]
fn hermes_session_request_builds_deterministic_command_invocation() {
    let request = HermesSessionRequest::new(
        "/repo/workspaces/task-a",
        "researcher",
        "work kanban task t_cb8f7a12",
    )
    .session_id("session-123")
    .skill("kanban-worker")
    .skill("test-driven-development")
    .toolset("terminal")
    .toolset("file")
    .approval_policy(HermesApprovalPolicy::Yolo);

    let invocation = request.command_invocation();

    assert_eq!(invocation.binary, "hermes");
    assert_eq!(
        invocation.working_directory.as_deref(),
        Some("/repo/workspaces/task-a")
    );
    assert_eq!(
        invocation.argv,
        vec![
            "--profile",
            "researcher",
            "--resume",
            "session-123",
            "--skills",
            "kanban-worker,test-driven-development",
            "--yolo",
            "chat",
            "--query",
            "work kanban task t_cb8f7a12",
            "--toolsets",
            "terminal,file",
            "--source",
            "openview-worker",
        ]
    );
    assert_eq!(invocation.env_keys, vec!["HERMES_HOME", "HERMES_PROFILE"]);
}

#[test]
fn hermes_session_request_omits_empty_optional_flags_and_keeps_approval_safe_by_default() {
    let invocation = HermesSessionRequest::new("/repo/workspaces/task-b", "writer", "draft brief")
        .command_invocation();

    assert_eq!(invocation.binary, "hermes");
    assert_eq!(
        invocation.working_directory.as_deref(),
        Some("/repo/workspaces/task-b")
    );
    assert_eq!(
        invocation.argv,
        vec![
            "--profile",
            "writer",
            "chat",
            "--query",
            "draft brief",
            "--source",
            "openview-worker",
        ]
    );
    assert!(!invocation.argv.iter().any(|arg| arg == "--yolo"));
    assert!(!invocation.argv.iter().any(|arg| arg == "--resume"));
    assert!(!invocation.argv.iter().any(|arg| arg == "--skills"));
    assert!(!invocation.argv.iter().any(|arg| arg == "--toolsets"));
    assert_eq!(invocation.env_keys, vec!["HERMES_HOME", "HERMES_PROFILE"]);
}

#[test]
fn hermes_session_request_serializes_lowercase_approval_policy_for_worker_contracts() {
    let request = HermesSessionRequest::new("/repo/workspaces/task-d", "researcher", "inspect")
        .approval_policy(HermesApprovalPolicy::Yolo);

    let value = serde_json::to_value(&request).expect("request serializes");

    assert_eq!(value["workspace"], "/repo/workspaces/task-d");
    assert_eq!(value["profile"], "researcher");
    assert_eq!(value["prompt"], "inspect");
    assert_eq!(value["approval_policy"], "yolo");
}

#[test]
fn hermes_command_invocation_serializes_only_secret_free_contract_fields() {
    let invocation = HermesSessionRequest::new("/repo/workspaces/task-c", "researcher", "inspect")
        .skill("kanban-worker")
        .toolset("terminal")
        .command_invocation();

    let value = serde_json::to_value(&invocation).expect("invocation serializes");

    assert_eq!(value["binary"], "hermes");
    assert_eq!(
        value["env_keys"],
        serde_json::json!(["HERMES_HOME", "HERMES_PROFILE"])
    );
    assert!(
        value.get("env").is_none(),
        "contract must expose env key names, not env values"
    );
    assert!(
        value.get("environment").is_none(),
        "contract must not serialize secret-bearing env maps"
    );
}
