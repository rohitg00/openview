use openview_core::{IiiBackendConfig, IiiDurableOperation, IiiQueuePrimitive, IiiRunMode};
use serde_json::json;

#[test]
fn iii_backend_contract_maps_absurd_durable_steps_to_iii_primitives() {
    let plan = IiiBackendConfig::default()
        .queue_name("agent-runs")
        .task_name("research-brief")
        .idempotency_key("spawn:research-brief:user-42")
        .worker_id("openview-worker-1")
        .session_id("session-42")
        .run_goal("write the research brief")
        .build_plan();

    assert_eq!(plan.queue_primitive, IiiQueuePrimitive::IiiQueuePreferred);
    assert_eq!(
        plan.function_id(IiiDurableOperation::SpawnTask),
        Some("research-brief")
    );
    assert_eq!(plan.function_id(IiiDurableOperation::ClaimLease), None);
    assert_eq!(
        plan.function_id(IiiDurableOperation::CheckpointStepCache),
        Some("iii-database::transaction")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::ReadCheckpointStepCache),
        Some("iii-database::query")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::AwaitEvent),
        Some("stream::list")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::EmitEvent),
        Some("stream::set")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::EmitEventFirstWins),
        Some("iii-database::execute")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::FanoutEvent),
        Some("hook-fanout::publish_collect")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::Sleep),
        Some("iii-database::execute")
    );
    assert_eq!(plan.function_id(IiiDurableOperation::Retry), None);
    assert_eq!(
        plan.function_id(IiiDurableOperation::Cancel),
        Some("iii::durable::publish")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::ListPendingApprovals),
        Some("approval::list_pending")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::ResolveApproval),
        Some("approval::resolve")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::EvidenceStreamAppend),
        Some("stream::set")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::EvidenceStreamRead),
        Some("stream::list")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::HermesRunStart),
        Some("run::start")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::HermesRunStartAndWait),
        Some("run::start_and_wait")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::SessionCreate),
        Some("session-tree::create")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::SessionAppend),
        Some("session-tree::append")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::SessionMessages),
        Some("session-tree::messages")
    );

    let spawn = plan.call(IiiDurableOperation::SpawnTask).unwrap();
    assert_eq!(spawn.payload["task"], json!("research-brief"));
    assert_eq!(
        plan.action(IiiDurableOperation::SpawnTask).unwrap(),
        &json!({"kind": "enqueue", "queue": "agent-runs"})
    );
    assert_eq!(
        spawn.payload["idempotency_key"],
        json!("spawn:research-brief:user-42")
    );
    assert!(spawn
        .semantics
        .iter()
        .any(|s| s.contains("spawn-time deduplication")));

    let checkpoint = plan.call(IiiDurableOperation::CheckpointStepCache).unwrap();
    assert_eq!(
        checkpoint.payload["table"],
        json!("durable_step_checkpoints")
    );
    assert_eq!(
        checkpoint.payload["statements"]
            .as_array()
            .expect("transaction statements")
            .len(),
        2
    );
    assert!(checkpoint
        .semantics
        .iter()
        .any(|s| s.contains("completed steps are cached and skipped on replay")));

    let event = plan.call(IiiDurableOperation::EmitEventFirstWins).unwrap();
    assert!(event.payload["sql"]
        .as_str()
        .expect("first-wins SQL")
        .contains("ON CONFLICT"));
    assert!(event
        .semantics
        .iter()
        .any(|s| s.contains("first emit for an event name wins")));

    let emitted = plan.call(IiiDurableOperation::EmitEvent).unwrap();
    assert_eq!(emitted.payload["stream_name"], json!("openview::events"));
    assert_eq!(emitted.payload["group_id"], json!("session-42"));
    assert!(emitted.payload.get("item_id").is_some());
    assert!(emitted.payload.get("data").is_some());
}

#[test]
fn iii_backend_uses_database_claim_lease_fallback_when_iii_queue_is_absent() {
    let plan = IiiBackendConfig::default()
        .without_iii_queue()
        .queue_name("agent-runs")
        .task_name("research-brief")
        .build_plan();

    assert_eq!(
        plan.queue_primitive,
        IiiQueuePrimitive::DatabaseLeaseFallback
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::SpawnTask),
        Some("iii-database::transaction")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::ClaimLease),
        Some("iii-database::transaction")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::Sleep),
        Some("iii-database::execute")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::Retry),
        Some("iii-database::execute")
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::Cancel),
        Some("iii-database::execute")
    );

    let claim = plan.call(IiiDurableOperation::ClaimLease).unwrap();
    assert_eq!(
        claim.function_id.as_deref(),
        Some("iii-database::transaction")
    );
    assert_eq!(claim.payload["lease_table"], json!("durable_task_leases"));
    assert_eq!(
        claim.payload["where"],
        json!("visible_at <= now and lease_expires_at < now")
    );
    assert!(claim
        .semantics
        .iter()
        .any(|s| s.contains("time-limited claim lease")));
}

#[test]
fn iii_backend_plans_hermes_run_modes_and_session_tree_evidence_without_live_network_calls() {
    let plan = IiiBackendConfig::default()
        .session_id("session-42")
        .run_goal("inspect the repo")
        .hermes_mode(IiiRunMode::StartAndWait)
        .build_plan();

    assert!(
        plan.planning_only,
        "contract planner must not perform live iii calls"
    );
    assert_eq!(
        plan.function_id(IiiDurableOperation::HermesRunStartAndWait),
        Some("run::start_and_wait")
    );
    assert_eq!(
        plan.call(IiiDurableOperation::HermesRunStartAndWait)
            .unwrap()
            .payload["messages"][0]["content"][0]["text"],
        json!("inspect the repo")
    );
    assert_eq!(
        plan.call(IiiDurableOperation::HermesRunStartAndWait)
            .unwrap()
            .payload["model"],
        json!("gpt-5.5")
    );
    assert_eq!(
        plan.call(IiiDurableOperation::HermesRunStartAndWait)
            .unwrap()
            .payload["reasoning_effort"],
        json!("xhigh")
    );
    assert_eq!(
        plan.call(IiiDurableOperation::SessionCreate)
            .unwrap()
            .payload["display_name"],
        json!("OpenView durable run session-42")
    );
    assert_eq!(
        plan.call(IiiDurableOperation::EvidenceStreamAppend)
            .unwrap()
            .payload["stream_name"],
        json!("openview::evidence")
    );
    assert_eq!(
        plan.call(IiiDurableOperation::SessionMessages)
            .unwrap()
            .payload["session_id"],
        json!("session-42")
    );
}
