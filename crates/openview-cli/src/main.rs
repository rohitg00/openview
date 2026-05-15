use anyhow::Result;
use clap::{Parser, Subcommand};
use openview_core::{
    built_in_worker_catalog, CancelRunRequest, CheckRunStatus, ConnectionGraph, DiffReviewComment,
    EventPage, LocalRunStore, MergeReadiness, OpenViewControlApi, OpenViewRuntime,
    PullRequestMetadata, RejectApprovalRequest, ResourceKind, ReviewWorkflow, RunPhase, RunRecord,
    TodoBlocker, WorkerRuntimeSpec,
};
use openview_worker::{
    approval_worker_manifest, claude_code_agent_worker_manifest, codex_agent_worker_manifest,
    git_worktree_worker_manifest, hermes_agent_worker_manifest, opencode_agent_worker_manifest,
    runtime_manifest_json, shell_sandbox_worker_manifest, terminal_pty_worker_manifest,
    OPENVIEW_QUEUE_INDEX_NAMES, OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION,
    OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID, OPENVIEW_QUEUE_SCHEMA_VERSION, OPENVIEW_QUEUE_TABLE_NAMES,
    WORKER_PROCESS_QUEUE_LEASE_HEARTBEAT_CONTROL_API_METHOD,
};
use serde_json::json;

#[derive(Debug, Parser)]
#[command(name = "openview")]
#[command(about = "Rust-first agent-worker orchestration control plane")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the built-in worker catalog as JSON.
    Catalog,
    /// Run a local approval-gated orchestration demo.
    Demo,
    /// Operator approval queue commands backed by deterministic demo data.
    Approvals {
        #[command(subcommand)]
        command: ApprovalCommand,
    },
    /// Print control-plane visibility snapshots as JSON.
    ControlPlane {
        #[command(subcommand)]
        kind: VisibilityKind,
    },
    /// Export demo evidence bundles for human review.
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// Print an individual worker manifest.
    Manifest { worker: String },
    /// Operator queue adapter readiness commands backed by deterministic demo data.
    Queues {
        #[command(subcommand)]
        command: QueueCommand,
    },
    /// Operator run trace commands backed by deterministic demo data.
    Runs {
        #[command(subcommand)]
        command: RunCommand,
    },
    /// Operator worker runtime commands backed by deterministic demo data.
    Workers {
        #[command(subcommand)]
        command: WorkerCommand,
    },
}

#[derive(Debug, Clone, Copy, Subcommand)]
enum ApprovalCommand {
    /// List open approvals for the demo runtime.
    List,
    /// Approve the blocked demo run and print the resulting state.
    ApproveDemo,
    /// Reject the blocked demo run and print the resulting failed state.
    RejectDemo,
}

#[derive(Debug, Clone, Copy, Subcommand)]
enum EvidenceCommand {
    /// Export a deterministic demo run evidence bundle as JSON.
    ExportDemo,
}

#[derive(Debug, Clone, Copy, Subcommand)]
enum QueueCommand {
    /// Print queue adapter production-readiness state.
    Readiness,
}

#[derive(Debug, Clone, Copy, Subcommand)]
enum RunCommand {
    /// Print demo run events in watch-friendly event order.
    Watch,
    /// Print a paged demo run event snapshot.
    Events,
    /// Cancel the demo run and print the resulting state.
    CancelDemo,
}

#[derive(Debug, Clone, Copy, Subcommand)]
enum WorkerCommand {
    /// Print demo worker runtime health state.
    Status,
}

#[derive(Debug, Clone, Copy, Subcommand)]
enum VisibilityKind {
    /// Print demo worker runtime health state.
    Runtime,
    /// Print a paged event-stream snapshot from the demo run.
    Events,
    /// Print open approval state for a blocked demo run.
    Approvals,
    /// Approve the blocked demo run and print resulting approval/events state.
    ApproveDemo,
    /// Print review workflow readiness state.
    Review,
    /// Print queue adapter production-readiness state.
    QueueReadiness,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Catalog => {
            println!(
                "{}",
                serde_json::to_string_pretty(&built_in_worker_catalog())?
            );
        }
        Command::Demo => {
            let (_, run) = demo_runtime_and_run()?;
            println!("{}", serde_json::to_string_pretty(&run)?);
        }
        Command::Approvals { command } => {
            println!("{}", approvals_command_json(command)?);
        }
        Command::ControlPlane { kind } => {
            println!("{}", control_plane_visibility_json(kind)?);
        }
        Command::Evidence { command } => match command {
            EvidenceCommand::ExportDemo => println!("{}", evidence_export_demo_json()?),
        },
        Command::Manifest { worker } => {
            let manifest = match worker.as_str() {
                "approval.gate" => approval_worker_manifest(),
                "codex.agent" => codex_agent_worker_manifest(),
                "claude-code.agent" => claude_code_agent_worker_manifest(),
                "git.worktree" => git_worktree_worker_manifest(),
                "hermes.agent" => hermes_agent_worker_manifest(),
                "opencode.agent" => opencode_agent_worker_manifest(),
                "shell.sandbox" => shell_sandbox_worker_manifest(),
                "terminal.pty" => terminal_pty_worker_manifest(),
                other => built_in_worker_catalog()
                    .into_iter()
                    .find(|candidate| candidate.name == other)
                    .ok_or_else(|| anyhow::anyhow!("unknown worker: {other}"))?,
            };
            println!("{}", runtime_manifest_json(&manifest)?);
        }
        Command::Queues { command } => {
            println!("{}", queues_command_json(command)?);
        }
        Command::Runs { command } => {
            println!("{}", runs_command_json(command)?);
        }
        Command::Workers { command } => {
            println!("{}", workers_command_json(command)?);
        }
    }
    Ok(())
}

fn approvals_command_json(command: ApprovalCommand) -> Result<String> {
    match command {
        ApprovalCommand::List => control_plane_visibility_json(VisibilityKind::Approvals),
        ApprovalCommand::ApproveDemo => control_plane_visibility_json(VisibilityKind::ApproveDemo),
        ApprovalCommand::RejectDemo => reject_demo_json(),
    }
}

fn workers_command_json(command: WorkerCommand) -> Result<String> {
    match command {
        WorkerCommand::Status => control_plane_visibility_json(VisibilityKind::Runtime),
    }
}

fn queues_command_json(command: QueueCommand) -> Result<String> {
    match command {
        QueueCommand::Readiness => control_plane_visibility_json(VisibilityKind::QueueReadiness),
    }
}

fn runs_command_json(command: RunCommand) -> Result<String> {
    match command {
        RunCommand::Watch | RunCommand::Events => {
            control_plane_visibility_json(VisibilityKind::Events)
        }
        RunCommand::CancelDemo => cancel_demo_json(),
    }
}

fn evidence_export_demo_json() -> Result<String> {
    let (runtime, run) = demo_runtime_and_run()?;
    let events: EventPage = runtime.events_since(run.id, 0, 50)?;
    let worker_runtime_health = runtime.worker_runtimes().cloned().collect::<Vec<_>>();
    let value = json!({
        "schema": "openview.evidence.demo.v1",
        "run": {
            "id": run.id,
            "phase": run.phase,
            "goal": run.goal,
            "workers": run.workers,
        },
        "events": events.events,
        "approvals": approval_state_json(&run),
        "worker_runtime_health": worker_runtime_health,
        "queue_adapter_readiness": queue_adapter_readiness_json_value(),
        "review_readiness": review_readiness_json(),
        "artifact_placeholders": [
            {
                "artifact_id": "demo-run-events",
                "kind": "event_stream",
                "status": "placeholder",
                "contains_secrets": false,
                "description": "Deterministic demo event stream for review evidence export."
            },
            {
                "artifact_id": "demo-worker-logs",
                "kind": "worker_logs",
                "status": "placeholder",
                "contains_secrets": false,
                "description": "Sanitized worker log slot; demo runtime records metadata only."
            }
        ]
    });
    Ok(serde_json::to_string_pretty(&value)?)
}

fn demo_control_api_with_run() -> Result<(OpenViewControlApi, RunRecord)> {
    let (_, run) = demo_runtime_and_run()?;
    let mut store = LocalRunStore::default();
    store.persist_run(run.clone())?;
    Ok((OpenViewControlApi::new(store), run))
}

fn reject_demo_json() -> Result<String> {
    let (mut api, blocked_run) = demo_control_api_with_run()?;
    let approval = blocked_run
        .approvals
        .first()
        .ok_or_else(|| anyhow::anyhow!("demo run did not request approval"))?;
    let rejected = api.reject_approval(RejectApprovalRequest {
        run_id: blocked_run.id,
        approval_id: approval.id,
        reason: "rejected by control-plane reject-demo".to_string(),
    })?;
    let resolved_approval = rejected
        .run
        .approvals
        .iter()
        .find(|candidate| candidate.id == approval.id)
        .ok_or_else(|| anyhow::anyhow!("rejected run lost approval record"))?;
    let value = json!({
        "blocked_run_id": blocked_run.id,
        "rejected_run_id": rejected.run.id,
        "approval_id": resolved_approval.id,
        "function_id": resolved_approval.function_id,
        "approved": resolved_approval.approved,
        "resolved": resolved_approval.resolved_at.is_some(),
        "phase": rejected.run.phase,
        "events": rejected.run.events,
    });
    Ok(serde_json::to_string_pretty(&value)?)
}

fn cancel_demo_json() -> Result<String> {
    let (mut api, run) = demo_control_api_with_run()?;
    let cancelled = api.cancel_run(CancelRunRequest {
        run_id: run.id,
        reason: "cancelled by control-plane cancel-demo".to_string(),
    })?;
    let value = json!({
        "run_id": cancelled.run.id,
        "phase": cancelled.run.phase,
        "cancelled": cancelled.run.phase == RunPhase::Cancelled,
        "events": cancelled.run.events,
    });
    Ok(serde_json::to_string_pretty(&value)?)
}

fn control_plane_visibility_json(kind: VisibilityKind) -> Result<String> {
    match kind {
        VisibilityKind::Runtime => {
            let (runtime, _) = demo_runtime_and_run()?;
            let runtimes = runtime.worker_runtimes().cloned().collect::<Vec<_>>();
            Ok(serde_json::to_string_pretty(&runtimes)?)
        }
        VisibilityKind::Events => {
            let (runtime, run) = demo_runtime_and_run()?;
            let page: EventPage = runtime.events_since(run.id, 0, 50)?;
            Ok(serde_json::to_string_pretty(&page)?)
        }
        VisibilityKind::Approvals => {
            let (_, run) = demo_runtime_and_run()?;
            Ok(serde_json::to_string_pretty(&approval_state_json(&run))?)
        }
        VisibilityKind::ApproveDemo => {
            let (mut runtime, blocked_run) = demo_runtime_and_run()?;
            let approval = blocked_run
                .approvals
                .first()
                .ok_or_else(|| anyhow::anyhow!("demo run did not request approval"))?;
            let approved_run = runtime.approve(
                blocked_run.id,
                approval.id,
                "approved by control-plane approve-demo",
            )?;
            let resolved_approval = approved_run
                .approvals
                .iter()
                .find(|candidate| candidate.id == approval.id)
                .ok_or_else(|| anyhow::anyhow!("approved run lost approval record"))?;
            let value = json!({
                "blocked_run_id": blocked_run.id,
                "approved_run_id": approved_run.id,
                "approval_id": resolved_approval.id,
                "function_id": resolved_approval.function_id,
                "approved": resolved_approval.approved,
                "resolved": resolved_approval.resolved_at.is_some(),
                "events": approved_run.events,
            });
            Ok(serde_json::to_string_pretty(&value)?)
        }
        VisibilityKind::Review => Ok(serde_json::to_string_pretty(&review_readiness_json())?),
        VisibilityKind::QueueReadiness => Ok(serde_json::to_string_pretty(
            &queue_adapter_readiness_json_value(),
        )?),
    }
}

fn queue_adapter_readiness_json_value() -> serde_json::Value {
    json!({
        "schema": "openview.queue_adapter_readiness.demo.v1",
        "mode": "deterministic_demo",
        "contains_secret_values": false,
        "production_integration": {
            "status": "partial",
            "primary_queue_target": "iii-queue",
            "durable_fallback_target": "iii-database",
            "adapter_binding": "not_connected_in_cli_demo",
            "ready_to_claim_production_parity": false,
            "requires_iii_on_path": false
        },
        "queue_schema": {
            "status": "bootstrap_plan_and_live_apply_pass",
            "schema_version": OPENVIEW_QUEUE_SCHEMA_VERSION,
            "migration_id": OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID,
            "bootstrap_function": OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION,
            "migration_surface": "idempotent_plan_applied_by_live_iii_database_e2e",
            "owned_migrations_required": true,
            "tables": OPENVIEW_QUEUE_TABLE_NAMES,
            "indexes": OPENVIEW_QUEUE_INDEX_NAMES,
            "required_records": [
                {
                    "name": "durable_tasks",
                    "status": "local_contract_and_schema_plan_pass",
                    "fields": [
                        "task_id",
                        "queue",
                        "task_name",
                        "payload_ref",
                        "payload_sha256",
                        "state",
                        "attempt",
                        "visible_at",
                        "lease_id",
                        "lease_owner",
                        "lease_expires_at",
                        "last_heartbeat_at",
                        "completed_at",
                        "failure_ref",
                        "dead_lettered_at"
                    ]
                },
                {
                    "name": "durable_task_events",
                    "status": "local_contract_and_schema_plan_pass",
                    "fields": [
                        "task_id",
                        "kind",
                        "payload",
                        "created_at"
                    ]
                },
                {
                    "name": "durable_dead_letters",
                    "status": "local_contract_and_schema_plan_pass",
                    "fields": [
                        "task_id",
                        "failure_ref",
                        "failure_kind",
                        "worker_id",
                        "created_at"
                    ]
                },
                {
                    "name": "durable_idempotency_keys",
                    "status": "schema_plan_pass",
                    "fields": [
                        "idempotency_key",
                        "task_id",
                        "created_at"
                    ]
                }
            ]
        },
        "worker_heartbeat_to_lease": {
            "status": "control_api_and_process_supervisor_call_site_pass",
            "runtime_heartbeat_contract": "modeled_by_worker_runtime",
            "queue_lease_heartbeat_contract": "heartbeat_worker_leases_extends_active_owned_leases",
            "process_supervisor_call_site": WORKER_PROCESS_QUEUE_LEASE_HEARTBEAT_CONTROL_API_METHOD,
            "live_process_worker_wiring": "call_site_contract_declared",
            "current_demo_worker": {
                "worker_id": "terminal.pty",
                "runtime_health": "ready",
                "active_queue_leases": 0,
                "heartbeat_extends_live_leases": false
            },
            "tested_behavior": [
                "worker heartbeat updates runtime health and metadata",
                "active owned leases in the requested queue are extended",
                "expired leases are not revived and remain recoverable",
                "process supervisor heartbeat events carry sanitized metadata and the control API call-site"
            ],
            "expected_policy": "healthy workers extend current leases; missed heartbeats let visibility timeout release work for recovery"
        },
        "concurrency_policy": {
            "status": "observable_snapshot_contract_passes",
            "snapshot_schema": "QueueConcurrencySnapshotResponse",
            "local_policy": {
                "policy_kind": "locally_enforced",
                "fields": [
                    "queue",
                    "requested_max_concurrency",
                    "active_lease_count",
                    "available_task_count",
                    "blocked_by_limit",
                    "locally_enforced"
                ],
                "blocking_rule": "blocked_by_limit is true when active_lease_count >= requested_max_concurrency"
            },
            "iii_queue_policy": {
                "policy_kind": "delegated_to_iii_queue",
                "locally_enforced": false,
                "blocking_rule": "OpenView reports state without claiming local enforcement"
            }
        },
        "live_iii_database_e2e": {
            "status": "gated_external_check",
            "last_run_status": "not_run_by_cli_demo",
            "managed_command": "OPENVIEW_III_E2E_DATABASE=1 OPENVIEW_III_MANAGED=1 scripts/e2e_iii_runtime.sh",
            "cargo_command": "OPENVIEW_RUN_LIVE_III_E2E=1 cargo test -p openview-worker --test live_iii_e2e -- --nocapture",
            "default_sqlite_path": "/private/tmp/openview-iii-e2e/openview-e2e.db",
            "sqlite_path_env": "$III_E2E_DEMO_DIR/openview-e2e.db",
            "lifecycle": [
                {
                    "step": "start_managed_stack",
                    "status": "script_gated"
                },
                {
                    "step": "start_iii_database",
                    "status": "enabled_by_OPENVIEW_III_E2E_DATABASE"
                },
                {
                    "step": "apply_owned_queue_schema_bootstrap",
                    "status": "verifies_durable_tables_indexes_and_columns"
                },
                {
                    "step": "apply_task_state_check",
                    "status": "verifies_transaction_query_execute_against_durable_tasks"
                },
                {
                    "step": "exercise_lease_heartbeat_recovery_retry_dead_letter",
                    "status": "verified_when_script_exits_zero"
                },
                {
                    "step": "stop_managed_stack",
                    "status": "handled_by_script_when_OPENVIEW_III_MANAGED_is_set"
                }
            ]
        },
        "operator_next_checks": [
            {
                "name": "local_cli_surface",
                "command": "cargo run -p openview-cli -- queues readiness",
                "expected_result": "deterministic JSON only"
            },
            {
                "name": "live_database_task_state",
                "command": "OPENVIEW_III_E2E_DATABASE=1 OPENVIEW_III_MANAGED=1 scripts/e2e_iii_runtime.sh",
                "expected_result": "database lease heartbeat recovery retry dead-letter checks pass"
            }
        ]
    })
}

fn review_readiness_json() -> serde_json::Value {
    let workflow = sample_review_workflow();
    let readiness = match workflow.merge_readiness() {
        MergeReadiness::Ready => "ready",
        MergeReadiness::Blocked => "blocked",
    };
    json!({
        "readiness": readiness,
        "summary": workflow.readiness_summary(),
        "workflow": workflow,
    })
}

fn approval_state_json(run: &RunRecord) -> serde_json::Value {
    let approvals = run
        .approvals
        .iter()
        .map(|approval| {
            json!({
                "approval_id": approval.id,
                "worker_id": approval.worker_id,
                "function_id": approval.function_id,
                "approved": approval.approved,
                "resolved": approval.resolved_at.is_some(),
                "reason": approval.reason,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "run_id": run.id,
        "blocked": run.phase == RunPhase::WaitingForApproval,
        "phase": run.phase,
        "approvals": approvals,
    })
}

fn demo_runtime_and_run() -> Result<(OpenViewRuntime, RunRecord)> {
    let mut runtime = OpenViewRuntime::default();
    runtime.register_worker(approval_worker_manifest())?;
    runtime.register_worker(shell_sandbox_worker_manifest())?;
    runtime.register_worker(git_worktree_worker_manifest())?;
    runtime.register_worker(terminal_pty_worker_manifest())?;
    runtime.register_worker_runtime(
        WorkerRuntimeSpec::new("terminal.pty", "openview-terminal-worker")
            .arg("--workspace")
            .env_key("OPENVIEW_WORKSPACE_ROOT")
            .ready(),
    )?;
    runtime.register_worker_runtime(
        WorkerRuntimeSpec::new("shell.sandbox", "openview-shell-worker")
            .env_key("OPENVIEW_SANDBOX_PROFILE")
            .start(),
    )?;
    runtime.heartbeat_worker_runtime("terminal.pty")?;
    runtime.fail_worker_runtime("shell.sandbox", "demo policy denied network access")?;
    let run = runtime.start_run(
        "prepare an interactive terminal session for safe review",
        ["terminal.pty", "shell.sandbox"],
    )?;
    Ok((runtime, run))
}

fn sample_review_workflow() -> ReviewWorkflow {
    ReviewWorkflow::new(
        PullRequestMetadata::new(42, "worker-runtime-visibility", "main")
            .title("Expose control-plane visibility from CLI")
            .author("openview"),
    )
    .review_comment(
        DiffReviewComment::new(
            "crates/openview-cli/src/main.rs",
            42,
            "show runtime and review state in CLI output",
        )
        .author("reviewer"),
    )
    .check_run(CheckRunStatus::failed("cargo test -p openview-cli"))
    .todo_blocker(TodoBlocker::new(
        "todo-cli-help",
        "verify the new control-plane subcommands appear in --help",
    ))
}

#[allow(dead_code)]
fn _graph_shape_smoke() -> Result<()> {
    let mut graph = ConnectionGraph::new("demo");
    graph.shared_resource(ResourceKind::EventStream);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn runtime_visibility_reports_registered_worker_health() {
        let value: Value =
            serde_json::from_str(&control_plane_visibility_json(VisibilityKind::Runtime).unwrap())
                .unwrap();
        let runtimes = value.as_array().expect("runtime output is a JSON array");

        assert!(runtimes.iter().any(|runtime| {
            runtime["worker_id"] == "terminal.pty" && runtime["health_state"] == "ready"
        }));
    }

    #[test]
    fn event_visibility_pages_demo_run_events() {
        let value: Value =
            serde_json::from_str(&control_plane_visibility_json(VisibilityKind::Events).unwrap())
                .unwrap();

        assert_eq!(value["has_more"], false);
        assert!(value["events"]
            .as_array()
            .expect("event output includes events")
            .iter()
            .any(|event| event["kind"] == "approval.requested"));
    }

    #[test]
    fn review_visibility_includes_readiness_summary() {
        let value: Value =
            serde_json::from_str(&control_plane_visibility_json(VisibilityKind::Review).unwrap())
                .unwrap();

        assert_eq!(value["readiness"], "blocked");
        assert_eq!(
            value["summary"],
            "blocked: 1 unresolved review comment, 1 failing check, 1 open todo blocker"
        );
    }

    #[test]
    fn approvals_visibility_reports_blocked_run_and_open_approval() {
        let value: Value = serde_json::from_str(
            &control_plane_visibility_json(VisibilityKind::Approvals).unwrap(),
        )
        .unwrap();

        assert_eq!(value["blocked"], true);
        assert!(value["run_id"].as_str().is_some());
        let approval = &value["approvals"][0];
        assert!(approval["approval_id"].as_str().is_some());
        assert_eq!(approval["function_id"], "terminal.pty::spawn");
        assert_eq!(approval["approved"], false);
        assert_eq!(approval["resolved"], false);
    }

    #[test]
    fn approve_demo_resolves_approval_and_returns_resulting_events() {
        let value: Value = serde_json::from_str(
            &control_plane_visibility_json(VisibilityKind::ApproveDemo).unwrap(),
        )
        .unwrap();

        assert_eq!(value["blocked_run_id"], value["approved_run_id"]);
        assert!(value["approval_id"].as_str().is_some());
        assert_eq!(value["function_id"], "terminal.pty::spawn");
        assert_eq!(value["approved"], true);
        assert_eq!(value["resolved"], true);
        assert_eq!(
            value["events"].as_array().unwrap().last().unwrap()["kind"],
            "run.completed"
        );
    }

    #[test]
    fn operator_parity_commands_are_exposed_as_top_level_shims() {
        assert!(Cli::try_parse_from(["openview", "approvals", "list"]).is_ok());
        assert!(Cli::try_parse_from(["openview", "approvals", "approve-demo"]).is_ok());
        assert!(Cli::try_parse_from(["openview", "approvals", "reject-demo"]).is_ok());
        assert!(Cli::try_parse_from(["openview", "workers", "status"]).is_ok());
        assert!(Cli::try_parse_from(["openview", "queues", "readiness"]).is_ok());
        assert!(Cli::try_parse_from(["openview", "control-plane", "queue-readiness"]).is_ok());
        assert!(Cli::try_parse_from(["openview", "runs", "watch"]).is_ok());
        assert!(Cli::try_parse_from(["openview", "runs", "events"]).is_ok());
        assert!(Cli::try_parse_from(["openview", "runs", "cancel-demo"]).is_ok());
        assert!(Cli::try_parse_from(["openview", "evidence", "export-demo"]).is_ok());
    }

    #[test]
    fn reject_demo_resolves_approval_and_fails_run_without_secret_payloads() {
        let value: Value = serde_json::from_str(&reject_demo_json().unwrap()).unwrap();

        assert_eq!(value["blocked_run_id"], value["rejected_run_id"]);
        assert!(value["approval_id"].as_str().is_some());
        assert_eq!(value["function_id"], "terminal.pty::spawn");
        assert_eq!(value["approved"], false);
        assert_eq!(value["resolved"], true);
        assert_eq!(value["phase"], "failed");
        let events = value["events"].as_array().unwrap();
        assert!(events
            .iter()
            .any(|event| event["kind"] == "approval.rejected"));
        assert_eq!(events.last().unwrap()["kind"], "run.failed");

        assert_no_secret_like_strings(&value);
    }

    #[test]
    fn cancel_demo_marks_run_cancelled_without_secret_payloads() {
        let value: Value = serde_json::from_str(&cancel_demo_json().unwrap()).unwrap();

        assert!(value["run_id"].as_str().is_some());
        assert_eq!(value["phase"], "cancelled");
        assert_eq!(value["cancelled"], true);
        assert!(value["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["kind"] == "run.cancelled"));

        assert_no_secret_like_strings(&value);
    }

    #[test]
    fn evidence_export_demo_includes_operator_review_bundle_without_secret_payloads() {
        let value: Value = serde_json::from_str(&evidence_export_demo_json().unwrap()).unwrap();

        assert_eq!(value["schema"], "openview.evidence.demo.v1");
        assert!(value["run"]["id"].as_str().is_some());
        assert!(value["events"].as_array().unwrap().len() >= 3);
        assert_eq!(value["approvals"]["blocked"], true);
        assert!(value["worker_runtime_health"]
            .as_array()
            .unwrap()
            .iter()
            .any(|runtime| {
                runtime["worker_id"] == "terminal.pty" && runtime["health_state"] == "ready"
            }));
        assert_eq!(value["review_readiness"]["readiness"], "blocked");
        assert!(value["artifact_placeholders"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| {
                artifact["artifact_id"] == "demo-run-events"
                    && artifact["contains_secrets"] == false
            }));

        assert_no_secret_like_strings(&value);
    }

    #[test]
    fn queue_readiness_reports_schema_heartbeat_and_live_e2e_state_without_secrets() {
        let value: Value = serde_json::from_str(
            &control_plane_visibility_json(VisibilityKind::QueueReadiness).unwrap(),
        )
        .unwrap();

        assert_eq!(value["schema"], "openview.queue_adapter_readiness.demo.v1");
        assert_eq!(value["production_integration"]["status"], "partial");
        assert_eq!(
            value["queue_schema"]["required_records"][0]["name"],
            "durable_tasks"
        );
        assert_eq!(
            value["queue_schema"]["bootstrap_function"],
            "openview.queue::bootstrap_schema"
        );
        assert_eq!(
            value["worker_heartbeat_to_lease"]["status"],
            "control_api_and_process_supervisor_call_site_pass"
        );
        assert_eq!(
            value["worker_heartbeat_to_lease"]["live_process_worker_wiring"],
            "call_site_contract_declared"
        );
        assert_eq!(
            value["concurrency_policy"]["status"],
            "observable_snapshot_contract_passes"
        );
        assert_eq!(
            value["live_iii_database_e2e"]["managed_command"],
            "OPENVIEW_III_E2E_DATABASE=1 OPENVIEW_III_MANAGED=1 scripts/e2e_iii_runtime.sh"
        );
        assert_eq!(value["contains_secret_values"], false);

        assert_no_secret_like_strings(&value);
    }

    fn assert_no_secret_like_strings(value: &Value) {
        let rendered = serde_json::to_string(value).unwrap();
        assert!(!rendered.contains("TOKEN"));
        assert!(!rendered.contains("SECRET"));
        assert!(!rendered.contains("PASSWORD"));
    }
}
