use chrono::{Duration, TimeZone, Utc};
use openview_core::{
    ApprovalRecord, ArtifactRecord, LocalRunStore, OpenViewError, QueueTask, QueueTaskStatus,
    RunEvent, RunPhase, RunRecord, StepRecord, WorkerHealthState, WorkerRuntimeSpec,
};
use serde_json::json;
use uuid::Uuid;

fn at(seconds: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0).single().unwrap()
}

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn event(sequence: u64, kind: &str) -> RunEvent {
    RunEvent {
        sequence,
        kind: kind.to_string(),
        at: at(100 + sequence as i64),
        payload: json!({"sequence": sequence}),
    }
}

fn approval(worker_id: &str) -> ApprovalRecord {
    ApprovalRecord {
        id: id(900),
        worker_id: worker_id.to_string(),
        function_id: "shell::run".to_string(),
        reason: "shell execution requires human review".to_string(),
        resolved_reason: Some("approved for evidence test".to_string()),
        approved: true,
        created_at: at(100),
        resolved_at: Some(at(140)),
    }
}

fn run_record(run_id: Uuid) -> RunRecord {
    RunRecord {
        id: run_id,
        goal: "export deterministic evidence bundle".to_string(),
        phase: RunPhase::Completed,
        workers: vec!["shell.sandbox".to_string(), "policy.guard".to_string()],
        steps: vec![StepRecord {
            id: id(901),
            worker_id: "shell.sandbox".to_string(),
            phase: RunPhase::Completed,
            function_ids: vec!["shell::run".to_string()],
        }],
        approvals: vec![approval("shell.sandbox")],
        events: Vec::new(),
    }
}

fn artifact(
    run_id: Uuid,
    artifact_id: &str,
    created_at: i64,
    payload_hint: &str,
) -> ArtifactRecord {
    ArtifactRecord::new(
        artifact_id,
        run_id,
        format!("artifact://{artifact_id}"),
        "application/json",
        128,
        format!("sha256-{artifact_id}"),
        at(created_at),
    )
    .metadata("safe_name", artifact_id)
    .metadata("payload_hint", payload_hint)
}

#[test]
fn evidence_bundle_export_is_deterministic_and_contains_only_contract_metadata() {
    let run_id = id(1);
    let mut store = LocalRunStore::default();
    store.persist_run(run_record(run_id)).unwrap();
    store
        .persist_event(run_id, event(1, "run.started"))
        .unwrap();
    store
        .persist_event(run_id, event(2, "approval.approved"))
        .unwrap();
    store
        .persist_event(run_id, event(3, "run.completed"))
        .unwrap();

    let mut task = QueueTask::new(
        "task-1",
        "shell.sandbox",
        json!({"run_id": run_id, "secret_prompt": "do not bundle payload"}),
        at(100),
    );
    let lease = task
        .lease("lease-1", "shell.sandbox", at(110), Duration::seconds(30))
        .unwrap();
    task.complete(lease.id, at(120)).unwrap();
    store.persist_queue_task(task).unwrap();
    store
        .persist_worker_runtime(WorkerRuntimeSpec::new("shell.sandbox", "openview-worker").ready())
        .unwrap();
    store
        .persist_worker_runtime(WorkerRuntimeSpec::new("unrelated.worker", "other-worker").ready())
        .unwrap();
    store
        .persist_artifact(artifact(run_id, "z-transcript", 220, "summary only"))
        .unwrap();
    store
        .persist_artifact(artifact(run_id, "a-log", 210, "redacted log"))
        .unwrap();

    let first = store.export_evidence_bundle(run_id).unwrap();
    let second = store.export_evidence_bundle(run_id).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.run.id, run_id);
    assert_eq!(first.run.phase, RunPhase::Completed);
    assert_eq!(first.approvals, first.run.approvals);
    assert_eq!(
        first
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        first
            .artifacts
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a-log", "z-transcript"]
    );
    assert_eq!(first.worker_runtimes.len(), 1);
    assert_eq!(first.worker_runtimes[0].worker_id, "shell.sandbox");
    assert_eq!(
        first.worker_runtimes[0].health_state,
        WorkerHealthState::Ready
    );
    assert_eq!(first.queue_tasks.len(), 1);
    assert_eq!(first.queue_tasks[0].id, "task-1");
    assert_eq!(first.queue_tasks[0].status, QueueTaskStatus::Completed);

    let serialized = serde_json::to_value(&first).unwrap();
    assert_eq!(
        serialized["queue_tasks"][0].get("payload"),
        None,
        "queue task summaries must not embed secret queue payloads"
    );
    assert_eq!(
        serialized["artifacts"][0].get("payload"),
        None,
        "artifact records expose metadata only, never secret payload bytes"
    );
    assert!(serialized.to_string().contains("redacted log"));
    assert!(!serialized.to_string().contains("do not bundle payload"));
}

#[test]
fn evidence_bundle_rejects_unknown_runs_and_gapful_events() {
    let run_id = id(2);
    let mut store = LocalRunStore::default();

    assert_eq!(
        store.export_evidence_bundle(run_id),
        Err(OpenViewError::RunNotFound(run_id))
    );

    store.persist_run(run_record(run_id)).unwrap();
    store
        .persist_event(run_id, event(1, "run.started"))
        .unwrap();
    let mut snapshot = store.snapshot();
    snapshot
        .run_events
        .get_mut(&run_id)
        .unwrap()
        .push(event(3, "gap"));
    let restarted = LocalRunStore::from_snapshot(snapshot);

    assert_eq!(
        restarted,
        Err(OpenViewError::RunEventSequenceGap {
            run_id,
            expected: 2,
            actual: 3,
        })
    );
}

#[test]
fn snapshot_restart_preserves_artifacts_for_bundle_export() {
    let run_id = id(3);
    let mut store = LocalRunStore::default();
    store.persist_run(run_record(run_id)).unwrap();
    store
        .persist_event(run_id, event(1, "run.started"))
        .unwrap();
    store
        .persist_artifact(artifact(run_id, "evidence-json", 300, "safe metadata"))
        .unwrap();

    let restarted = LocalRunStore::from_snapshot(store.snapshot()).unwrap();
    let artifacts = restarted.artifacts_for_run(run_id).unwrap();
    let bundle = restarted.export_evidence_bundle(run_id).unwrap();

    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].id, "evidence-json");
    assert_eq!(bundle.artifacts, artifacts);
}
