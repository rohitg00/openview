use openview_worker::{
    AgentCliApprovalPolicy, AgentCliKind, AgentCliSessionRequest, HermesApprovalPolicy,
    HermesSessionRequest,
};

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

#[test]
fn codex_agent_cli_invocation_runs_exec_in_the_assigned_worktree() {
    let invocation = AgentCliSessionRequest::new(
        AgentCliKind::Codex,
        "/repo/.openview-worktrees/codex-a",
        "build the dashboard and run tests",
    )
    .model("gpt-5.5")
    .profile("xhigh")
    .command_invocation();

    assert_eq!(invocation.worker_id, "codex.agent");
    assert_eq!(invocation.binary, "codex");
    assert_eq!(
        invocation.working_directory.as_deref(),
        Some("/repo/.openview-worktrees/codex-a")
    );
    assert_eq!(
        invocation.argv,
        vec![
            "exec",
            "--cd",
            "/repo/.openview-worktrees/codex-a",
            "--sandbox",
            "workspace-write",
            "--ask-for-approval",
            "on-request",
            "--json",
            "--model",
            "gpt-5.5",
            "--profile",
            "xhigh",
            "build the dashboard and run tests",
        ]
    );
    assert_eq!(invocation.env_keys, vec!["CODEX_HOME", "OPENAI_API_KEY"]);
}

#[test]
fn claude_code_agent_cli_invocation_uses_print_mode_without_secret_values() {
    let invocation = AgentCliSessionRequest::new(
        AgentCliKind::ClaudeCode,
        "/repo/.openview-worktrees/claude-a",
        "review the queue recovery path",
    )
    .model("claude-opus-4-7")
    .approval_policy(AgentCliApprovalPolicy::Yolo)
    .command_invocation();

    assert_eq!(invocation.worker_id, "claude-code.agent");
    assert_eq!(invocation.binary, "claude");
    assert_eq!(
        invocation.argv,
        vec![
            "--model",
            "claude-opus-4-7",
            "--output-format",
            "stream-json",
            "--dangerously-skip-permissions",
            "-p",
            "review the queue recovery path",
        ]
    );
    assert_eq!(
        invocation.env_keys,
        vec!["ANTHROPIC_API_KEY", "CLAUDE_CONFIG_DIR"]
    );
    let value = serde_json::to_value(&invocation).expect("invocation serializes");
    assert!(value.get("env").is_none());
    assert!(value.get("environment").is_none());
}

#[test]
fn hermes_and_opencode_generic_agent_invocations_share_agentview_contract() {
    let hermes = AgentCliSessionRequest::new(
        AgentCliKind::Hermes,
        "/repo/.openview-worktrees/hermes-a",
        "work kanban task t_123",
    )
    .profile("builder")
    .session_id("hermes-session-1")
    .skill("kanban-worker")
    .toolset("terminal")
    .command_invocation();

    assert_eq!(hermes.worker_id, "hermes.agent");
    assert_eq!(hermes.binary, "hermes");
    assert_eq!(
        hermes.argv,
        vec![
            "--profile",
            "builder",
            "--resume",
            "hermes-session-1",
            "--skills",
            "kanban-worker",
            "--toolsets",
            "terminal",
            "chat",
            "--query",
            "work kanban task t_123",
        ]
    );

    let opencode = AgentCliSessionRequest::new(
        AgentCliKind::OpenCode,
        "/repo/.openview-worktrees/opencode-a",
        "implement the file tree",
    )
    .model("qwen-coder")
    .session_id("open-session-1")
    .command_invocation();

    assert_eq!(opencode.worker_id, "opencode.agent");
    assert_eq!(opencode.binary, "opencode");
    assert_eq!(
        opencode.argv,
        vec![
            "run",
            "--model",
            "qwen-coder",
            "--session",
            "open-session-1",
            "implement the file tree",
        ]
    );
}

fn assert_invocation_has_no_secret_values(invocation: &openview_worker::AgentCliCommandInvocation) {
    let value = serde_json::to_value(invocation).expect("invocation serializes");
    assert!(value.get("env").is_none());
    assert!(value.get("environment").is_none());

    let encoded = value.to_string();
    for secret_like_value in [
        "sk-test-secret",
        "anthropic-test-secret",
        "google-test-secret",
        "cursor-test-secret",
    ] {
        assert!(
            !encoded.contains(secret_like_value),
            "invocation must not embed secret values"
        );
    }
}

#[test]
fn additional_agent_cli_invocations_build_runner_specific_commands() {
    let cases = vec![
        (
            AgentCliSessionRequest::new(
                AgentCliKind::OpenClaw,
                "/repo/.openview-worktrees/openclaw-a",
                "coordinate gateway task",
            )
            .model("gpt-5")
            .profile("control")
            .session_id("openclaw-session-1")
            .command_invocation(),
            "openclaw.agent",
            "openclaw",
            vec![
                "--profile",
                "control",
                "agent",
                "--json",
                "--model",
                "gpt-5",
                "--session-id",
                "openclaw-session-1",
                "--message",
                "coordinate gateway task",
            ],
        ),
        (
            AgentCliSessionRequest::new(
                AgentCliKind::GeminiCli,
                "/repo/.openview-worktrees/gemini-a",
                "inspect the planner",
            )
            .model("gemini-2.5-pro")
            .session_id("gemini-session-1")
            .command_invocation(),
            "gemini-cli.agent",
            "gemini",
            vec![
                "--model",
                "gemini-2.5-pro",
                "--resume",
                "gemini-session-1",
                "-p",
                "inspect the planner",
            ],
        ),
        (
            AgentCliSessionRequest::new(
                AgentCliKind::Goose,
                "/repo/.openview-worktrees/goose-a",
                "fix the queue test",
            )
            .model("claude-sonnet-4")
            .session_id("goose-session-1")
            .command_invocation(),
            "goose.agent",
            "goose",
            vec![
                "run",
                "--model",
                "claude-sonnet-4",
                "--resume",
                "goose-session-1",
                "-t",
                "fix the queue test",
            ],
        ),
        (
            AgentCliSessionRequest::new(
                AgentCliKind::Aider,
                "/repo/.openview-worktrees/aider-a",
                "update the docs",
            )
            .model("openai/gpt-5")
            .approval_policy(AgentCliApprovalPolicy::Yolo)
            .command_invocation(),
            "aider.agent",
            "aider",
            vec![
                "--model",
                "openai/gpt-5",
                "--yes-always",
                "--message",
                "update the docs",
            ],
        ),
        (
            AgentCliSessionRequest::new(
                AgentCliKind::OpenHands,
                "/repo/.openview-worktrees/openhands-a",
                "repair the migration",
            )
            .model("anthropic/claude-sonnet-4")
            .session_id("openhands-session-1")
            .command_invocation(),
            "openhands.agent",
            "openhands",
            vec![
                "--headless",
                "--model",
                "anthropic/claude-sonnet-4",
                "--resume",
                "openhands-session-1",
                "-t",
                "repair the migration",
            ],
        ),
        (
            AgentCliSessionRequest::new(
                AgentCliKind::Crush,
                "/repo/.openview-worktrees/crush-a",
                "simplify the reducer",
            )
            .model("qwen3-coder")
            .session_id("crush-session-1")
            .command_invocation(),
            "crush.agent",
            "crush",
            vec![
                "run",
                "--model",
                "qwen3-coder",
                "--session",
                "crush-session-1",
                "simplify the reducer",
            ],
        ),
        (
            AgentCliSessionRequest::new(
                AgentCliKind::QwenCode,
                "/repo/.openview-worktrees/qwen-a",
                "implement the parser",
            )
            .model("qwen3-coder")
            .command_invocation(),
            "qwen-code.agent",
            "qwen",
            vec!["--model", "qwen3-coder", "-p", "implement the parser"],
        ),
        (
            AgentCliSessionRequest::new(
                AgentCliKind::CursorAgent,
                "/repo/.openview-worktrees/cursor-a",
                "review the patch",
            )
            .session_id("cursor-session-1")
            .command_invocation(),
            "cursor-agent.agent",
            "cursor-agent",
            vec![
                "--resume",
                "cursor-session-1",
                "--output-format",
                "json",
                "-p",
                "review the patch",
            ],
        ),
        (
            AgentCliSessionRequest::new(
                AgentCliKind::Amp,
                "/repo/.openview-worktrees/amp-a",
                "complete the refactor",
            )
            .model("claude-sonnet-4")
            .session_id("amp-session-1")
            .command_invocation(),
            "amp.agent",
            "amp",
            vec![
                "--model",
                "claude-sonnet-4",
                "--resume",
                "amp-session-1",
                "--execute",
                "complete the refactor",
            ],
        ),
    ];

    for (invocation, worker_id, binary, expected_argv) in cases {
        assert_eq!(invocation.worker_id, worker_id);
        assert_eq!(invocation.binary, binary);
        assert_eq!(
            invocation.argv,
            expected_argv
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        );
        assert_invocation_has_no_secret_values(&invocation);
    }
}

#[test]
fn goose_and_openhands_can_build_acp_mode_commands() {
    let goose = AgentCliSessionRequest::new(
        AgentCliKind::Goose,
        "/repo/.openview-worktrees/goose-acp",
        "ignored by acp server mode",
    )
    .toolset("acp")
    .command_invocation();

    assert_eq!(goose.worker_id, "goose.agent");
    assert_eq!(goose.binary, "goose");
    assert_eq!(goose.argv, vec!["acp"]);
    assert_invocation_has_no_secret_values(&goose);

    let openhands = AgentCliSessionRequest::new(
        AgentCliKind::OpenHands,
        "/repo/.openview-worktrees/openhands-acp",
        "ignored by acp server mode",
    )
    .toolset("acp")
    .command_invocation();

    assert_eq!(openhands.worker_id, "openhands.agent");
    assert_eq!(openhands.binary, "openhands");
    assert_eq!(openhands.argv, vec!["acp"]);
    assert_invocation_has_no_secret_values(&openhands);
}
