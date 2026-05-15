use openview_core::{
    git_worktree_worker_manifest, terminal_pty_worker_manifest, AgentBlueprint, BackendTarget,
    CapabilityRisk, ConnectionGraph, ExecutionMode, FunctionSpec, FunctionVisibility,
    OpenViewRuntime, PermissionPolicy, ResourceKind, RunPhase, SandboxProfile, TriggerSpec,
    WorkerManifest,
};

#[test]
fn built_in_catalog_contains_first_class_git_worktree_worker() {
    let catalog = openview_core::built_in_worker_catalog();
    let manifest = catalog
        .iter()
        .find(|worker| worker.name == "git.worktree")
        .expect("git.worktree is registered in the built-in catalog");

    assert_eq!(manifest, &git_worktree_worker_manifest());
}

#[test]
fn built_in_catalog_contains_first_class_terminal_pty_worker() {
    let catalog = openview_core::built_in_worker_catalog();
    let manifest = catalog
        .iter()
        .find(|worker| worker.name == "terminal.pty")
        .expect("terminal.pty is registered in the built-in catalog");

    assert_eq!(manifest, &terminal_pty_worker_manifest());
}

#[test]
fn terminal_pty_manifest_declares_locked_down_contract() {
    let manifest = terminal_pty_worker_manifest();

    assert_eq!(manifest.name, "terminal.pty");
    assert!(manifest.targets.contains(&BackendTarget::LocalBinary));
    assert!(manifest.resources.contains(&ResourceKind::Terminal));
    assert!(manifest.resources.contains(&ResourceKind::Process));
    assert!(manifest.resources.contains(&ResourceKind::EventStream));
    assert!(manifest.resources.contains(&ResourceKind::Filesystem));

    let sandbox = manifest.sandbox.as_ref().expect("sandbox declared");
    assert_eq!(sandbox.name, "locked-down");
    assert!(sandbox
        .filesystem
        .read_roots
        .contains("${OPENVIEW_WORKSPACE_ROOT}"));
    assert!(sandbox
        .filesystem
        .write_roots
        .contains("${OPENVIEW_WORKSPACE_ROOT}"));
    assert!(!sandbox.network.allow);
    assert!(sandbox.process.allow_exec);
    assert!(sandbox
        .process
        .allowed_commands
        .contains("${OPENVIEW_SAFE_SHELL}"));
    assert!(sandbox
        .process
        .allowed_commands
        .contains("${OPENVIEW_SAFE_AGENT_CLI}"));

    let function = |id: &str| {
        manifest
            .functions
            .iter()
            .find(|function| function.id == id)
            .unwrap_or_else(|| panic!("missing function {id}"))
    };

    let spawn = function("terminal.pty::spawn");
    assert_eq!(spawn.risk, CapabilityRisk::Exec);
    assert!(spawn.approval_required);
    assert_eq!(spawn.request_schema["required"][0], "command");
    assert_eq!(spawn.response_schema["required"][0], "session_id");

    let write = function("terminal.pty::write");
    assert_eq!(write.risk, CapabilityRisk::Exec);
    assert!(write.approval_required);

    let read = function("terminal.pty::read");
    assert_eq!(read.risk, CapabilityRisk::Read);
    assert!(!read.approval_required);

    let resize = function("terminal.pty::resize");
    assert_eq!(resize.risk, CapabilityRisk::State);
    assert!(!resize.approval_required);

    let kill = function("terminal.pty::kill");
    assert_eq!(kill.risk, CapabilityRisk::Exec);
    assert!(kill.approval_required);

    for id in [
        "terminal.pty::spawn",
        "terminal.pty::write",
        "terminal.pty::kill",
    ] {
        assert!(manifest.security.approval_required_functions.contains(id));
    }

    for function in &manifest.functions {
        assert_eq!(function.request_schema["type"], "object");
        assert!(function.request_schema.get("properties").is_some());
        assert_eq!(function.response_schema["type"], "object");
        assert!(function.response_schema.get("properties").is_some());
    }
}

#[test]
fn git_worktree_manifest_declares_locked_down_contract() {
    let manifest = git_worktree_worker_manifest();

    assert_eq!(manifest.name, "git.worktree");
    assert!(manifest.targets.contains(&BackendTarget::LocalBinary));
    assert!(manifest.resources.contains(&ResourceKind::GitWorktree));
    assert!(manifest.resources.contains(&ResourceKind::Filesystem));
    assert!(manifest.resources.contains(&ResourceKind::Process));

    let sandbox = manifest.sandbox.as_ref().expect("sandbox declared");
    assert_eq!(sandbox.name, "locked-down");
    assert!(sandbox
        .filesystem
        .read_roots
        .contains("${OPENVIEW_WORKSPACE_ROOT}"));
    assert!(sandbox
        .filesystem
        .read_roots
        .contains("${OPENVIEW_WORKTREE_ROOT}"));
    assert!(sandbox
        .filesystem
        .write_roots
        .contains("${OPENVIEW_WORKTREE_ROOT}"));
    assert!(!sandbox.network.allow);
    assert!(sandbox.process.allow_exec);
    assert!(sandbox.process.allowed_commands.contains("git"));

    let function = |id: &str| {
        manifest
            .functions
            .iter()
            .find(|function| function.id == id)
            .unwrap_or_else(|| panic!("missing function {id}"))
    };

    for id in ["git.worktree::list", "git.worktree::status"] {
        let function = function(id);
        assert_eq!(function.risk, CapabilityRisk::Read);
        assert!(!function.approval_required);
        assert_eq!(function.visibility, FunctionVisibility::Ui);
    }
    for id in ["git.worktree::create", "git.worktree::remove"] {
        let function = function(id);
        assert_eq!(function.risk, CapabilityRisk::Write);
        assert!(function.approval_required);
        assert!(manifest.security.approval_required_functions.contains(id));
    }

    for function in &manifest.functions {
        assert_eq!(function.request_schema["type"], "object");
        assert!(function.request_schema.get("properties").is_some());
        assert_eq!(function.response_schema["type"], "object");
        assert!(function.response_schema.get("properties").is_some());
    }
}

#[test]
fn registers_rust_workers_with_capabilities_resources_and_sandbox_policy() {
    let mut runtime = OpenViewRuntime::default();
    let manifest = WorkerManifest::new("shell.sandbox", "0.1.0")
        .description("Runs scoped commands inside an approval-aware sandbox")
        .target(BackendTarget::LocalBinary)
        .function(FunctionSpec::new("shell::run", CapabilityRisk::Exec).approval_required(true))
        .trigger(TriggerSpec::durable_subscriber(
            "shell::run",
            "agent::command_requested",
        ))
        .resource(ResourceKind::Sandbox)
        .resource(ResourceKind::Filesystem)
        .sandbox(
            SandboxProfile::locked_down()
                .allow_workspace_read("/workspace")
                .deny_network(),
        );

    runtime
        .register_worker(manifest)
        .expect("worker registered");
    let worker = runtime.registry().worker("shell.sandbox").unwrap();

    assert_eq!(worker.functions[0].id, "shell::run");
    assert_eq!(worker.security.default_policy, PermissionPolicy::Deny);
    assert!(worker
        .security
        .approval_required_functions
        .contains(&"shell::run".to_string()));
    assert!(worker.resources.contains(&ResourceKind::Sandbox));
    assert!(!worker.sandbox.as_ref().unwrap().network.allow);
}

#[test]
fn builds_agent_graph_that_connects_planner_approval_sandbox_and_model_workers() {
    let mut graph = ConnectionGraph::new("repo-hardening");
    graph
        .agent(AgentBlueprint::new("planner").uses_worker("models.router"))
        .agent(AgentBlueprint::new("builder").uses_worker("shell.sandbox"))
        .agent(AgentBlueprint::new("reviewer").uses_worker("policy.guard"))
        .handoff("planner", "builder")
        .handoff("builder", "reviewer")
        .approval_gate("builder", "approval.gate")
        .shared_resource(ResourceKind::SessionState)
        .shared_resource(ResourceKind::EventStream);

    let plan = graph.compile().expect("graph compiles");
    assert_eq!(plan.execution_mode, ExecutionMode::DurableQueued);
    assert_eq!(plan.edges.len(), 2);
    assert!(plan.required_workers.contains(&"approval.gate".to_string()));
    assert!(plan.shared_resources.contains(&ResourceKind::EventStream));
}

#[test]
fn run_state_machine_blocks_on_approval_then_resumes_to_completed() {
    let mut runtime = OpenViewRuntime::default();
    runtime
        .register_worker(
            WorkerManifest::new("approval.gate", "0.1.0").function(FunctionSpec::new(
                "approval::request",
                CapabilityRisk::Approval,
            )),
        )
        .unwrap();
    runtime
        .register_worker(
            WorkerManifest::new("shell.sandbox", "0.1.0")
                .function(
                    FunctionSpec::new("shell::run", CapabilityRisk::Exec).approval_required(true),
                )
                .sandbox(SandboxProfile::locked_down()),
        )
        .unwrap();

    let run = runtime
        .start_run("ship safely", ["shell.sandbox"])
        .expect("run starts");
    assert_eq!(run.phase, RunPhase::WaitingForApproval);
    assert_eq!(run.approvals.len(), 1);

    let resumed = runtime
        .approve(run.id, run.approvals[0].id, "approved after inspection")
        .unwrap();
    assert_eq!(resumed.phase, RunPhase::Completed);
    assert!(resumed
        .events
        .iter()
        .any(|event| event.kind == "approval.approved"));
    assert!(resumed
        .events
        .iter()
        .any(|event| event.kind == "run.completed"));
}

#[test]
fn emits_three_i_compatible_registration_messages_without_engine_dependency() {
    let manifest = WorkerManifest::new("storage.index", "0.1.0")
        .function(FunctionSpec::new("storage::put", CapabilityRisk::Write))
        .trigger(TriggerSpec::http("storage::put", "/storage/put"));

    let messages = manifest.to_backend_messages();
    let json = serde_json::to_value(messages).unwrap();

    assert_eq!(json[0]["type"], "registerworker");
    assert_eq!(json[1]["type"], "registerfunction");
    assert_eq!(json[2]["type"], "registertrigger");
    assert_eq!(json[2]["config"]["api_path"], "/storage/put");
}

#[test]
fn workspace_session_models_control_plane_tabs_panes_and_worktrees() {
    use openview_core::{PaneKind, WorkspacePane, WorkspaceSession, WorkspaceTab, WorktreeBinding};

    let session = WorkspaceSession::new("agent review room")
        .tab(
            WorkspaceTab::new("tab-runs", "Runs")
                .pane(WorkspacePane::new(
                    "pane-trace",
                    PaneKind::TraceTimeline,
                    "Trace",
                ))
                .pane(WorkspacePane::new(
                    "pane-approvals",
                    PaneKind::ApprovalQueue,
                    "Approvals",
                )),
        )
        .worktree(WorktreeBinding::new(
            "wt-openview",
            "/repo/openview",
            "main",
        ));

    session.validate().expect("workspace session is restorable");
    assert_eq!(
        session.tabs[0].active_pane_id.as_deref(),
        Some("pane-trace")
    );
    assert_eq!(session.worktrees[0].branch, "main");
}

#[test]
fn worker_runtime_spec_tracks_binary_lifecycle_and_safe_env_key_names_only() {
    use openview_core::{WorkerHealthState, WorkerRuntimeSpec};

    let runtime = WorkerRuntimeSpec::new("shell.sandbox", "openview-worker-shell")
        .arg("--manifest")
        .env_key("OPENVIEW_WORKER_CONFIG")
        .ready();

    assert_eq!(runtime.health_state, WorkerHealthState::Ready);
    assert_eq!(runtime.env_keys, vec!["OPENVIEW_WORKER_CONFIG"]);
    assert!(runtime.started_at.is_some());
    assert!(runtime.last_heartbeat_at.is_some());
    assert!(runtime.stopped_at.is_none());
    assert_eq!(runtime.restart_count, 0);
}

#[test]
fn openview_runtime_supervises_worker_start_heartbeat_fail_restart_and_stop() {
    use openview_core::{WorkerHealthState, WorkerRuntimeEventKind, WorkerRuntimeSpec};

    let mut runtime = OpenViewRuntime::default();
    runtime
        .register_worker_runtime(
            WorkerRuntimeSpec::new("shell.sandbox", "openview-worker-shell")
                .arg("--stdio")
                .env_key("OPENVIEW_WORKER_CONFIG"),
        )
        .expect("runtime registered");

    let started = runtime
        .start_worker_runtime("shell.sandbox")
        .expect("worker started");
    let started_at = started.started_at.expect("start timestamp recorded");
    assert_eq!(started.health_state, WorkerHealthState::Starting);
    assert!(started.last_heartbeat_at.is_none());

    let ready = runtime
        .heartbeat_worker_runtime("shell.sandbox")
        .expect("worker heartbeat accepted");
    assert_eq!(ready.health_state, WorkerHealthState::Ready);
    assert!(ready.last_heartbeat_at.expect("heartbeat recorded") >= started_at);

    let failed = runtime
        .fail_worker_runtime("shell.sandbox", "child process exited")
        .expect("worker failure recorded");
    let failed_at = failed
        .last_failure
        .as_ref()
        .expect("failure details recorded")
        .failed_at;
    assert_eq!(failed.health_state, WorkerHealthState::Failed);
    assert_eq!(
        failed.last_failure.as_ref().unwrap().reason,
        "child process exited"
    );

    let restarted = runtime
        .restart_worker_runtime("shell.sandbox")
        .expect("worker restarted");
    assert_eq!(restarted.health_state, WorkerHealthState::Starting);
    assert_eq!(restarted.restart_count, 1);
    assert!(restarted.started_at.expect("restart timestamp recorded") >= failed_at);
    assert_eq!(
        restarted.last_failure.as_ref().unwrap().reason,
        "child process exited"
    );

    let ready_again = runtime
        .heartbeat_worker_runtime("shell.sandbox")
        .expect("worker heartbeat accepted after restart");
    assert_eq!(ready_again.health_state, WorkerHealthState::Ready);
    assert_eq!(ready_again.restart_count, 1);

    let stopped = runtime
        .stop_worker_runtime("shell.sandbox")
        .expect("worker stopped");
    assert_eq!(stopped.health_state, WorkerHealthState::Stopped);
    assert!(stopped.stopped_at.is_some());
    assert_eq!(
        runtime
            .worker_runtime("shell.sandbox")
            .expect("runtime exists")
            .health_state,
        WorkerHealthState::Stopped
    );

    let events = runtime.worker_runtime_events();
    assert_eq!(
        events.iter().map(|event| event.kind).collect::<Vec<_>>(),
        vec![
            WorkerRuntimeEventKind::Started,
            WorkerRuntimeEventKind::Heartbeat,
            WorkerRuntimeEventKind::Failed,
            WorkerRuntimeEventKind::Restarted,
            WorkerRuntimeEventKind::Heartbeat,
            WorkerRuntimeEventKind::Stopped,
        ]
    );
    assert_eq!(events[0].sequence, 1);
    assert_eq!(events[2].reason.as_deref(), Some("child process exited"));
    assert_eq!(events[5].state, WorkerHealthState::Stopped);
}
