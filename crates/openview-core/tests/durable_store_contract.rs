use chrono::{Duration, TimeZone, Utc};
use openview_core::{
    ApprovalRecord, LocalRunStore, OpenViewError, QueueEventKind, QueueTask, QueueTaskStatus,
    RunEvent, RunPhase, RunRecord, StepRecord, WorkerRuntimeSpec, WorkflowDefinition, WorkflowNode,
};
use serde_json::json;
use uuid::Uuid;

fn at(seconds: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0).single().unwrap()
}

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn approval(worker_id: &str, approved: bool) -> ApprovalRecord {
    ApprovalRecord {
        id: id(900),
        worker_id: worker_id.to_string(),
        function_id: "shell::run".to_string(),
        reason: "shell execution requires human review".to_string(),
        resolved_reason: approved.then(|| "approved by operator".to_string()),
        approved,
        created_at: at(100),
        resolved_at: approved.then(|| at(140)),
    }
}

fn waiting_run(run_id: Uuid) -> RunRecord {
    RunRecord {
        id: run_id,
        goal: "ship durable replay".to_string(),
        phase: RunPhase::WaitingForApproval,
        workers: vec!["shell.sandbox".to_string(), "policy.guard".to_string()],
        steps: vec![StepRecord {
            id: id(901),
            worker_id: "shell.sandbox".to_string(),
            phase: RunPhase::WaitingForApproval,
            function_ids: vec!["shell::run".to_string()],
        }],
        approvals: vec![approval("shell.sandbox", false)],
        events: Vec::new(),
    }
}

fn event(sequence: u64, kind: &str) -> RunEvent {
    RunEvent {
        sequence,
        kind: kind.to_string(),
        at: at(100 + sequence as i64),
        payload: json!({"sequence": sequence}),
    }
}

fn workflow_definition() -> WorkflowDefinition {
    WorkflowDefinition::new("durable.workflow", "1", "Durable replay")
        .node(WorkflowNode::action("start", "run::start"))
        .node(WorkflowNode::approval("approve", "approval::request").depends_on("start"))
}

#[test]
fn local_run_store_persists_runs_events_definitions_and_queries_by_runtime_state() {
    let run_id = id(1);
    let mut store = LocalRunStore::default();
    let workflow = workflow_definition();
    let run = waiting_run(run_id);

    store
        .persist_workflow_definition(workflow.clone())
        .expect("valid workflow definition persists");
    store.persist_run(run.clone()).expect("run persists");
    store
        .persist_worker_runtime(WorkerRuntimeSpec::new("shell.sandbox", "openview-worker").start())
        .expect("worker runtime persists");
    store
        .persist_event(run_id, event(1, "run.started"))
        .expect("first event persists");
    store
        .persist_event(run_id, event(2, "approval.requested"))
        .expect("next event persists");

    let replayed = store
        .replay_run_state(run_id)
        .expect("persisted run can be replayed");

    assert_eq!(
        store.workflow_definition("durable.workflow"),
        Some(&workflow)
    );
    assert_eq!(replayed.phase, RunPhase::WaitingForApproval);
    assert_eq!(replayed.approvals, run.approvals);
    assert_eq!(
        replayed
            .events
            .iter()
            .map(|e| e.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(store.runs_for_worker("shell.sandbox"), vec![&run]);
    assert_eq!(store.approvals_by_state(false), vec![&run.approvals[0]]);
    assert_eq!(
        store.worker_runtime("shell.sandbox").unwrap().binary,
        "openview-worker"
    );
}

#[test]
fn persist_event_rejects_non_monotonic_or_gapful_run_sequences() {
    let run_id = id(2);
    let mut store = LocalRunStore::default();
    store.persist_run(waiting_run(run_id)).unwrap();
    store
        .persist_event(run_id, event(1, "run.started"))
        .unwrap();

    let duplicate = store.persist_event(run_id, event(1, "duplicate"));
    assert_eq!(
        duplicate,
        Err(OpenViewError::RunEventSequenceGap {
            run_id,
            expected: 2,
            actual: 1,
        })
    );

    let skipped = store.persist_event(run_id, event(3, "approval.requested"));
    assert_eq!(
        skipped,
        Err(OpenViewError::RunEventSequenceGap {
            run_id,
            expected: 2,
            actual: 3,
        })
    );
}

#[test]
fn restart_snapshot_restores_phase_approvals_events_and_current_queue_status() {
    let run_id = id(3);
    let mut store = LocalRunStore::default();
    let mut task = QueueTask::new("task-1", "worker.run", json!({"run_id": run_id}), at(100));
    let lease = task
        .lease("lease-1", "shell.sandbox", at(110), Duration::seconds(30))
        .unwrap();

    store
        .persist_workflow_definition(workflow_definition())
        .unwrap();
    store.persist_run(waiting_run(run_id)).unwrap();
    store
        .persist_event(run_id, event(1, "run.started"))
        .unwrap();
    store
        .persist_event(run_id, event(2, "approval.requested"))
        .unwrap();
    store.persist_queue_task(task.clone()).unwrap();
    store
        .persist_worker_runtime(WorkerRuntimeSpec::new("shell.sandbox", "openview-worker").ready())
        .unwrap();

    let restarted =
        LocalRunStore::from_snapshot(store.snapshot()).expect("snapshot validates on restart");
    let replayed = restarted.replay_run_state(run_id).unwrap();

    assert_eq!(replayed.phase, RunPhase::WaitingForApproval);
    assert!(!replayed.approvals[0].approved);
    assert_eq!(replayed.events.len(), 2);
    assert_eq!(
        restarted.queue_task("task-1").unwrap().status,
        QueueTaskStatus::Leased
    );
    assert_eq!(
        restarted.queue_task("task-1").unwrap().lease.as_ref(),
        Some(&lease)
    );
    assert_eq!(
        restarted.queue_task("task-1").unwrap().events[1].kind,
        QueueEventKind::Leased
    );
}

#[test]
fn restart_snapshot_rejects_gapful_persisted_event_streams_before_replay() {
    let run_id = id(4);
    let mut store = LocalRunStore::default();
    store.persist_run(waiting_run(run_id)).unwrap();
    store
        .persist_event(run_id, event(1, "run.started"))
        .unwrap();

    let mut snapshot = store.snapshot();
    snapshot
        .run_events
        .get_mut(&run_id)
        .unwrap()
        .push(event(3, "gap"));

    assert_eq!(
        LocalRunStore::from_snapshot(snapshot),
        Err(OpenViewError::RunEventSequenceGap {
            run_id,
            expected: 2,
            actual: 3,
        })
    );
}
