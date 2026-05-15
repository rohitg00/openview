use chrono::{Duration, TimeZone, Utc};
use openview_core::{
    ApprovalRecord, CompleteTaskRequest, EnqueueTaskRequest, EventCursor, FailTaskRequest,
    HeartbeatWorkerLeaseRequest, LocalRunStore, OpenViewControlApi, OpenViewError, PollWorkRequest,
    QueueEventKind, QueueTask, QueueTaskStatus, ReadEventsRequest, RecoverExpiredLeasesRequest,
    RegisterWorkflowDefinitionRequest, RunEvent, RunPhase, RunRecord, StepRecord,
    WorkerHealthState, WorkerRuntimeSpec, WorkflowDefinition, WorkflowNode,
};
use serde_json::json;
use uuid::Uuid;

fn at(seconds: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0).single().unwrap()
}

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn workflow_definition() -> WorkflowDefinition {
    WorkflowDefinition::new("control.workflow", "1", "Control workflow")
        .node(WorkflowNode::action("start", "run::start"))
        .node(WorkflowNode::approval("approve", "approval::request").depends_on("start"))
}

fn event(sequence: u64, kind: &str) -> RunEvent {
    RunEvent {
        sequence,
        kind: kind.to_string(),
        at: at(100 + sequence as i64),
        payload: json!({"sequence": sequence}),
    }
}

fn leased_task(
    task_id: &str,
    queue: &str,
    lease_id: &str,
    worker_id: &str,
    leased_at: i64,
    timeout_secs: i64,
) -> QueueTask {
    let mut task = QueueTask::new(task_id, queue, json!({"task_id": task_id}), at(100));
    task.lease(
        lease_id,
        worker_id,
        at(leased_at),
        Duration::seconds(timeout_secs),
    )
    .expect("test task can be leased");
    task
}

fn waiting_run(run_id: Uuid, approval_id: Uuid) -> RunRecord {
    RunRecord {
        id: run_id,
        goal: "approve release".to_string(),
        phase: RunPhase::WaitingForApproval,
        workers: vec!["shell.sandbox".to_string()],
        steps: vec![StepRecord {
            id: id(901),
            worker_id: "shell.sandbox".to_string(),
            phase: RunPhase::WaitingForApproval,
            function_ids: vec!["shell::run".to_string()],
        }],
        approvals: vec![ApprovalRecord {
            id: approval_id,
            worker_id: "shell.sandbox".to_string(),
            function_id: "shell::run".to_string(),
            reason: "shell execution requires review".to_string(),
            resolved_reason: None,
            approved: false,
            created_at: at(100),
            resolved_at: None,
        }],
        events: Vec::new(),
    }
}

#[test]
fn typed_control_api_registers_workflows_and_reads_ordered_event_pages() {
    let run_id = id(1);
    let mut store = LocalRunStore::default();
    store.persist_run(waiting_run(run_id, id(900))).unwrap();
    store
        .persist_event(run_id, event(1, "run.started"))
        .unwrap();
    store
        .persist_event(run_id, event(2, "approval.requested"))
        .unwrap();
    store
        .persist_event(run_id, event(3, "run.waiting"))
        .unwrap();
    let mut api = OpenViewControlApi::new(store);

    let registered = api
        .register_workflow_definition(RegisterWorkflowDefinitionRequest {
            definition: workflow_definition(),
        })
        .expect("valid workflow registers");
    let page = api
        .read_events(ReadEventsRequest {
            cursor: EventCursor {
                run_id,
                after_sequence: 1,
                limit: 2,
            },
        })
        .expect("event page reads");

    assert_eq!(registered.definition_id, "control.workflow");
    assert_eq!(registered.version, "1");
    assert_eq!(
        page.events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(page.next_after_sequence, 3);
    assert!(!page.has_more);
}

#[test]
fn worker_polling_only_leases_visible_tasks_compatible_with_that_worker() {
    let mut api = OpenViewControlApi::default();
    api.enqueue_task(EnqueueTaskRequest {
        task_id: "task-shell".to_string(),
        queue: "shell.sandbox".to_string(),
        payload: json!({"command": "cargo test"}),
        visible_at: at(100),
    })
    .unwrap();
    api.enqueue_task(EnqueueTaskRequest {
        task_id: "task-policy".to_string(),
        queue: "policy.guard".to_string(),
        payload: json!({"policy": "deny"}),
        visible_at: at(100),
    })
    .unwrap();

    let polled = api
        .poll_work(PollWorkRequest {
            worker_id: "shell.sandbox".to_string(),
            lease_id: "lease-shell".to_string(),
            now: at(110),
            visibility_timeout: Duration::seconds(30),
        })
        .expect("compatible visible work leases");

    assert_eq!(
        polled.task.as_ref().map(|task| task.id.as_str()),
        Some("task-shell")
    );
    assert_eq!(
        polled.lease.as_ref().map(|lease| lease.id.as_str()),
        Some("lease-shell")
    );
    assert_eq!(polled.task.unwrap().queue, "shell.sandbox");
    assert_eq!(
        api.store().queue_task("task-policy").unwrap().status,
        QueueTaskStatus::Available
    );
}

#[test]
fn worker_lease_heartbeat_extends_multiple_active_leases_for_worker_queue_only() {
    let mut api = OpenViewControlApi::default();
    api.store_mut()
        .persist_worker_runtime(WorkerRuntimeSpec::new(
            "process.worker",
            "openview-process-worker",
        ))
        .unwrap();
    api.store_mut()
        .persist_worker_runtime(WorkerRuntimeSpec::new("other.worker", "other-worker"))
        .unwrap();

    for task in [
        leased_task(
            "task-a",
            "agent.queue",
            "lease-a",
            "process.worker",
            110,
            40,
        ),
        leased_task(
            "task-b",
            "agent.queue",
            "lease-b",
            "process.worker",
            111,
            40,
        ),
        leased_task("task-c", "agent.queue", "lease-c", "other.worker", 112, 50),
        leased_task(
            "task-d",
            "other.queue",
            "lease-d",
            "process.worker",
            113,
            50,
        ),
        QueueTask::new("task-e", "agent.queue", json!({}), at(100)),
    ] {
        api.store_mut().persist_queue_task(task).unwrap();
    }

    let response = api
        .heartbeat_worker_leases(HeartbeatWorkerLeaseRequest {
            worker_id: "process.worker".to_string(),
            queue: "agent.queue".to_string(),
            heartbeat_at: at(125),
            visibility_timeout: Duration::seconds(60),
            metadata: Some(json!({"phase": "running", "pid": 4242})),
        })
        .expect("worker heartbeat extends active owned queue leases");

    assert_eq!(
        response
            .tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        vec!["task-a", "task-b"]
    );
    assert_eq!(
        response
            .leases
            .iter()
            .map(|lease| (lease.id.as_str(), lease.expires_at))
            .collect::<Vec<_>>(),
        vec![("lease-a", at(185)), ("lease-b", at(185))]
    );
    assert_eq!(response.runtime.health_state, WorkerHealthState::Ready);
    assert_eq!(response.runtime.last_heartbeat_at, Some(at(125)));
    assert_eq!(
        response.runtime.last_heartbeat_metadata.as_ref(),
        Some(&json!({"phase": "running", "pid": 4242}))
    );

    for (task_id, lease_id) in [("task-a", "lease-a"), ("task-b", "lease-b")] {
        let task = api.store().queue_task(task_id).unwrap();
        assert_eq!(task.visible_at, at(185));
        assert_eq!(task.lease.as_ref().unwrap().expires_at, at(185));
        assert_eq!(task.events.last().unwrap().sequence, 3);
        assert_eq!(
            task.events.last().unwrap().kind,
            QueueEventKind::Heartbeated
        );
        assert_eq!(task.events.last().unwrap().at, at(125));
        assert_eq!(
            task.events.last().unwrap().lease_id.as_deref(),
            Some(lease_id)
        );
        assert_eq!(
            task.events.last().unwrap().worker_id.as_deref(),
            Some("process.worker")
        );
    }

    assert_eq!(
        api.store()
            .queue_task("task-c")
            .unwrap()
            .events
            .last()
            .unwrap()
            .kind,
        QueueEventKind::Leased
    );
    assert_eq!(
        api.store()
            .queue_task("task-c")
            .unwrap()
            .lease
            .as_ref()
            .unwrap()
            .expires_at,
        at(162)
    );
    assert_eq!(
        api.store()
            .queue_task("task-d")
            .unwrap()
            .events
            .last()
            .unwrap()
            .kind,
        QueueEventKind::Leased
    );
    assert_eq!(api.store().queue_task("task-e").unwrap().events.len(), 1);
    assert!(api
        .store()
        .worker_runtime("other.worker")
        .unwrap()
        .last_heartbeat_at
        .is_none());
}

#[test]
fn worker_lease_heartbeat_does_not_extend_expired_leases_and_they_remain_recoverable() {
    let mut api = OpenViewControlApi::default();
    api.store_mut()
        .persist_worker_runtime(WorkerRuntimeSpec::new(
            "process.worker",
            "openview-process-worker",
        ))
        .unwrap();
    api.store_mut()
        .persist_queue_task(leased_task(
            "expired-task",
            "agent.queue",
            "expired-lease",
            "process.worker",
            100,
            10,
        ))
        .unwrap();

    let response = api
        .heartbeat_worker_leases(HeartbeatWorkerLeaseRequest {
            worker_id: "process.worker".to_string(),
            queue: "agent.queue".to_string(),
            heartbeat_at: at(120),
            visibility_timeout: Duration::seconds(60),
            metadata: Some(json!({"phase": "late"})),
        })
        .expect("worker heartbeat succeeds without reviving expired leases");

    assert!(response.tasks.is_empty());
    assert!(response.leases.is_empty());
    assert_eq!(response.runtime.last_heartbeat_at, Some(at(120)));
    let expired = api.store().queue_task("expired-task").unwrap();
    assert_eq!(expired.status, QueueTaskStatus::Leased);
    assert_eq!(expired.lease.as_ref().unwrap().expires_at, at(110));
    assert_eq!(expired.events.len(), 2);
    assert_eq!(expired.events.last().unwrap().kind, QueueEventKind::Leased);

    let recovered = api
        .recover_expired_leases(RecoverExpiredLeasesRequest {
            queue: Some("agent.queue".to_string()),
            now: at(120),
        })
        .expect("expired lease is still recoverable");

    assert_eq!(
        recovered
            .tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        vec!["expired-task"]
    );
    let recovered = api.store().queue_task("expired-task").unwrap();
    assert_eq!(recovered.status, QueueTaskStatus::Available);
    assert!(recovered.lease.is_none());
    assert_eq!(
        recovered.events.last().unwrap().kind,
        QueueEventKind::LeaseExpired
    );
}

#[test]
fn worker_lease_heartbeat_updates_runtime_progress_metadata_without_tasks() {
    let mut api = OpenViewControlApi::default();
    api.store_mut()
        .persist_worker_runtime(WorkerRuntimeSpec::new(
            "process.worker",
            "openview-process-worker",
        ))
        .unwrap();

    let first = api
        .heartbeat_worker_leases(HeartbeatWorkerLeaseRequest {
            worker_id: "process.worker".to_string(),
            queue: "empty.queue".to_string(),
            heartbeat_at: at(200),
            visibility_timeout: Duration::seconds(60),
            metadata: Some(json!({"phase": "startup", "completed": 0, "total": 3})),
        })
        .expect("worker heartbeat can update runtime progress without leases");

    assert!(first.tasks.is_empty());
    assert_eq!(first.runtime.health_state, WorkerHealthState::Ready);
    assert_eq!(first.runtime.started_at, Some(at(200)));
    assert_eq!(first.runtime.last_heartbeat_at, Some(at(200)));
    assert_eq!(
        first.runtime.last_heartbeat_metadata.as_ref(),
        Some(&json!({"phase": "startup", "completed": 0, "total": 3}))
    );

    let second = api
        .heartbeat_worker_leases(HeartbeatWorkerLeaseRequest {
            worker_id: "process.worker".to_string(),
            queue: "empty.queue".to_string(),
            heartbeat_at: at(230),
            visibility_timeout: Duration::seconds(60),
            metadata: Some(json!({"phase": "indexing", "completed": 2, "total": 3})),
        })
        .expect("worker progress metadata is replaced by the latest heartbeat");

    assert_eq!(second.runtime.started_at, Some(at(200)));
    assert_eq!(second.runtime.last_heartbeat_at, Some(at(230)));
    assert_eq!(
        second.runtime.last_heartbeat_metadata.as_ref(),
        Some(&json!({"phase": "indexing", "completed": 2, "total": 3}))
    );
    assert_eq!(
        api.store()
            .worker_runtime("process.worker")
            .unwrap()
            .last_heartbeat_metadata
            .as_ref(),
        Some(&json!({"phase": "indexing", "completed": 2, "total": 3}))
    );
}

#[test]
fn completing_and_failing_tasks_require_current_lease_and_preserve_dead_letter_semantics() {
    let mut api = OpenViewControlApi::default();
    api.enqueue_task(EnqueueTaskRequest {
        task_id: "task-1".to_string(),
        queue: "shell.sandbox".to_string(),
        payload: json!({}),
        visible_at: at(100),
    })
    .unwrap();
    api.poll_work(PollWorkRequest {
        worker_id: "shell.sandbox".to_string(),
        lease_id: "lease-1".to_string(),
        now: at(110),
        visibility_timeout: Duration::seconds(30),
    })
    .unwrap();

    let stale = api.complete_task(CompleteTaskRequest {
        task_id: "task-1".to_string(),
        lease_id: "stale-lease".to_string(),
        completed_at: at(120),
    });
    assert_eq!(
        stale,
        Err(OpenViewError::QueueLeaseMismatch {
            task_id: "task-1".to_string(),
            expected: "lease-1".to_string(),
            actual: "stale-lease".to_string(),
        })
    );
    assert_eq!(
        api.store().queue_task("task-1").unwrap().status,
        QueueTaskStatus::Leased
    );

    let completed = api
        .complete_task(CompleteTaskRequest {
            task_id: "task-1".to_string(),
            lease_id: "lease-1".to_string(),
            completed_at: at(121),
        })
        .unwrap();
    assert_eq!(completed.task.status, QueueTaskStatus::Completed);

    api.enqueue_task(EnqueueTaskRequest {
        task_id: "task-2".to_string(),
        queue: "shell.sandbox".to_string(),
        payload: json!({}),
        visible_at: at(130),
    })
    .unwrap();
    api.poll_work(PollWorkRequest {
        worker_id: "shell.sandbox".to_string(),
        lease_id: "lease-2".to_string(),
        now: at(130),
        visibility_timeout: Duration::seconds(30),
    })
    .unwrap();
    let failed = api
        .fail_task(FailTaskRequest {
            task_id: "task-2".to_string(),
            lease_id: "lease-2".to_string(),
            reason: "permanent".to_string(),
            failed_at: at(131),
            max_retries: 1,
            retry_delay: Duration::seconds(5),
        })
        .unwrap();

    assert_eq!(failed.task.status, QueueTaskStatus::Failed);
    assert_eq!(
        failed.task.events.last().unwrap().kind,
        QueueEventKind::Failed
    );
}

#[test]
fn approval_control_resumes_waiting_runs_or_returns_clear_typed_error() {
    let run_id = id(2);
    let approval_id = id(902);
    let mut store = LocalRunStore::default();
    store.persist_run(waiting_run(run_id, approval_id)).unwrap();
    let mut api = OpenViewControlApi::new(store);

    let approved = api
        .approve_approval(openview_core::ApproveApprovalRequest {
            run_id,
            approval_id,
            reason: "safe to run".to_string(),
        })
        .expect("pending approval resumes run");

    assert_eq!(approved.run.phase, RunPhase::Completed);
    assert!(approved.run.approvals[0].approved);
    assert_eq!(approved.run.events.last().unwrap().kind, "run.completed");

    let repeated = api.approve_approval(openview_core::ApproveApprovalRequest {
        run_id,
        approval_id,
        reason: "already approved".to_string(),
    });
    assert_eq!(
        repeated,
        Err(OpenViewError::RunNotAwaitingApproval { run_id })
    );
}

#[test]
fn rejection_and_cancellation_record_terminal_phases() {
    let run_id = id(3);
    let approval_id = id(903);
    let mut store = LocalRunStore::default();
    store.persist_run(waiting_run(run_id, approval_id)).unwrap();
    let mut api = OpenViewControlApi::new(store);

    let rejected = api
        .reject_approval(openview_core::RejectApprovalRequest {
            run_id,
            approval_id,
            reason: "unsafe".to_string(),
        })
        .expect("pending approval can be rejected");
    assert_eq!(rejected.run.phase, RunPhase::Failed);
    assert_eq!(rejected.run.events.last().unwrap().kind, "run.failed");

    let cancel_run_id = id(4);
    api.store_mut()
        .persist_run(RunRecord {
            id: cancel_run_id,
            goal: "cancel me".to_string(),
            phase: RunPhase::Running,
            workers: vec!["shell.sandbox".to_string()],
            steps: Vec::new(),
            approvals: Vec::new(),
            events: Vec::new(),
        })
        .unwrap();

    let cancelled = api
        .cancel_run(openview_core::CancelRunRequest {
            run_id: cancel_run_id,
            reason: "operator requested".to_string(),
        })
        .expect("running run can be cancelled");

    assert_eq!(cancelled.run.phase, RunPhase::Cancelled);
    assert_eq!(cancelled.run.events.last().unwrap().kind, "run.cancelled");
}
