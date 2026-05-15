use openview_core::{
    AgentBlueprint, BackendTarget, CapabilityRisk, ConnectionGraph, ExecutionMode, FunctionSpec,
    OpenViewRuntime, PermissionPolicy, ResourceKind, RunPhase, SandboxProfile, TriggerSpec,
    WorkerManifest,
};

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
    assert!(runtime.last_heartbeat_at.is_some());
}
