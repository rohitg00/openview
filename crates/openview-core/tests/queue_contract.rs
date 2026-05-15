use chrono::{Duration, TimeZone, Utc};
use openview_core::{
    EnqueueTaskRequest, OpenViewControlApi, PollWorkRequest, QueueConcurrencyPolicyKind,
    QueueConcurrencySnapshotRequest, QueueEventKind, QueueLease, QueueTask, QueueTaskStatus,
    RecoverExpiredLeasesRequest,
};
use serde_json::json;

fn at(seconds: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0).single().unwrap()
}

#[test]
fn queue_task_starts_available_with_payload_and_no_retry_debt() {
    let task = QueueTask::new("task-1", "worker.run", json!({"goal": "ship"}), at(100));

    assert_eq!(task.id, "task-1");
    assert_eq!(task.queue, "worker.run");
    assert_eq!(task.payload, json!({"goal": "ship"}));
    assert_eq!(task.status, QueueTaskStatus::Available);
    assert_eq!(task.retry_count, 0);
    assert_eq!(task.visible_at, at(100));
    assert_eq!(task.lease, None);
    assert_eq!(task.events.len(), 1);
    assert_eq!(task.events[0].sequence, 1);
    assert_eq!(task.events[0].kind, QueueEventKind::Enqueued);
}

#[test]
fn lease_claim_hides_task_until_visibility_timeout_and_emits_deterministic_event() {
    let mut task = QueueTask::new("task-1", "worker.run", json!({"goal": "ship"}), at(100));

    let lease = task
        .lease("lease-1", "worker-a", at(110), Duration::seconds(30))
        .expect("available task can be leased");

    assert_eq!(
        lease,
        QueueLease::new("lease-1", "worker-a", at(110), at(140))
    );
    assert_eq!(task.status, QueueTaskStatus::Leased);
    assert_eq!(task.visible_at, at(140));
    assert_eq!(task.lease.as_ref(), Some(&lease));
    assert!(!task.is_visible_at(at(139)));
    assert!(task.is_visible_at(at(140)));
    assert_eq!(task.events[1].sequence, 2);
    assert_eq!(task.events[1].kind, QueueEventKind::Leased);
    assert_eq!(task.events[1].lease_id.as_deref(), Some("lease-1"));
    assert_eq!(task.events[1].worker_id.as_deref(), Some("worker-a"));
}

#[test]
fn heartbeat_extends_current_lease_visibility_and_recovery_deadline() {
    let mut task = QueueTask::new("task-1", "worker.run", json!({}), at(100));
    task.lease("lease-1", "worker-a", at(110), Duration::seconds(30))
        .unwrap();

    let lease = task
        .heartbeat("lease-1", at(125), Duration::seconds(40))
        .expect("current lease can heartbeat before expiry");

    assert_eq!(
        lease,
        QueueLease::new("lease-1", "worker-a", at(110), at(165))
    );
    assert_eq!(task.visible_at, at(165));
    assert_eq!(task.lease.as_ref(), Some(&lease));
    assert!(!task.is_visible_at(at(164)));
    assert!(task.is_visible_at(at(165)));
    assert_eq!(task.events[2].sequence, 3);
    assert_eq!(task.events[2].kind, QueueEventKind::Heartbeated);
    assert_eq!(task.events[2].lease_id.as_deref(), Some("lease-1"));
    assert_eq!(task.events[2].worker_id.as_deref(), Some("worker-a"));

    assert!(!task.release_expired_lease(at(164)));
    assert!(task.release_expired_lease(at(165)));
    assert_eq!(task.status, QueueTaskStatus::Available);
    assert_eq!(
        task.events.last().unwrap().kind,
        QueueEventKind::LeaseExpired
    );
}

#[test]
fn ack_complete_requires_current_lease_and_terminally_completes_task() {
    let mut task = QueueTask::new("task-1", "worker.run", json!({}), at(100));
    task.lease("lease-1", "worker-a", at(110), Duration::seconds(30))
        .unwrap();

    let rejected = task.ack("other-lease", at(120));
    assert!(rejected.is_err());
    assert_eq!(task.status, QueueTaskStatus::Leased);

    task.ack("lease-1", at(121)).expect("current lease can ack");
    assert_eq!(task.status, QueueTaskStatus::Completed);
    assert_eq!(task.lease, None);
    assert_eq!(task.completed_at, Some(at(121)));
    assert_eq!(task.events.last().unwrap().sequence, 3);
    assert_eq!(task.events.last().unwrap().kind, QueueEventKind::Completed);
}

#[test]
fn fail_with_retry_budget_requeues_with_incremented_retry_count_and_visibility_delay() {
    let mut task = QueueTask::new("task-1", "worker.run", json!({}), at(100));
    task.lease("lease-1", "worker-a", at(110), Duration::seconds(30))
        .unwrap();

    task.fail(
        "lease-1",
        "transient network",
        at(120),
        3,
        Duration::seconds(10),
    )
    .expect("leased task can fail and retry");

    assert_eq!(task.status, QueueTaskStatus::Available);
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.visible_at, at(130));
    assert_eq!(task.lease, None);
    assert_eq!(task.last_error.as_deref(), Some("transient network"));
    assert_eq!(task.events[2].sequence, 3);
    assert_eq!(task.events[2].kind, QueueEventKind::Failed);
    assert_eq!(task.events[3].sequence, 4);
    assert_eq!(task.events[3].kind, QueueEventKind::Requeued);
}

#[test]
fn fail_without_retry_budget_dead_letters_task_without_requeueing() {
    let mut task = QueueTask::new("task-1", "worker.run", json!({}), at(100));
    task.lease("lease-1", "worker-a", at(110), Duration::seconds(30))
        .unwrap();

    task.fail("lease-1", "permanent", at(120), 1, Duration::seconds(10))
        .unwrap();

    assert_eq!(task.status, QueueTaskStatus::Failed);
    assert_eq!(task.retry_count, 1);
    assert_eq!(task.visible_at, at(120));
    assert_eq!(task.lease, None);
    assert_eq!(task.last_error.as_deref(), Some("permanent"));
    assert_eq!(task.events.last().unwrap().kind, QueueEventKind::Failed);
}

#[test]
fn explicit_requeue_requires_current_lease_and_preserves_retry_count() {
    let mut task = QueueTask::new("task-1", "worker.run", json!({}), at(100));
    task.lease("lease-1", "worker-a", at(110), Duration::seconds(30))
        .unwrap();

    task.requeue("lease-1", at(115), Duration::seconds(5))
        .expect("leased task can be requeued");

    assert_eq!(task.status, QueueTaskStatus::Available);
    assert_eq!(task.retry_count, 0);
    assert_eq!(task.visible_at, at(120));
    assert_eq!(task.lease, None);
    assert_eq!(task.events.last().unwrap().sequence, 3);
    assert_eq!(task.events.last().unwrap().kind, QueueEventKind::Requeued);
}

#[test]
fn expired_lease_can_be_reclaimed_without_threads_or_runtime() {
    let mut task = QueueTask::new("task-1", "worker.run", json!({}), at(100));
    task.lease("lease-1", "worker-a", at(110), Duration::seconds(30))
        .unwrap();

    assert!(task
        .lease("lease-2", "worker-b", at(139), Duration::seconds(30))
        .is_err());
    task.release_expired_lease(at(140));
    let lease = task
        .lease("lease-2", "worker-b", at(141), Duration::seconds(20))
        .expect("expired lease returns to available state");

    assert_eq!(lease.worker_id, "worker-b");
    assert_eq!(task.status, QueueTaskStatus::Leased);
    assert_eq!(task.retry_count, 0);
    assert_eq!(task.events[2].kind, QueueEventKind::LeaseExpired);
    assert_eq!(task.events[3].kind, QueueEventKind::Leased);
}

#[test]
fn control_api_recovers_expired_leases_in_bulk_for_requested_queue() {
    let mut api = OpenViewControlApi::default();
    for (task_id, queue) in [
        ("task-1", "worker.run"),
        ("task-2", "worker.run"),
        ("task-3", "other.run"),
    ] {
        api.enqueue_task(EnqueueTaskRequest {
            task_id: task_id.to_string(),
            queue: queue.to_string(),
            payload: json!({}),
            visible_at: at(100),
        })
        .unwrap();
    }

    api.poll_work(PollWorkRequest {
        worker_id: "worker.run".to_string(),
        lease_id: "lease-1".to_string(),
        now: at(110),
        visibility_timeout: Duration::seconds(10),
    })
    .unwrap();
    api.poll_work(PollWorkRequest {
        worker_id: "worker.run".to_string(),
        lease_id: "lease-2".to_string(),
        now: at(111),
        visibility_timeout: Duration::seconds(9),
    })
    .unwrap();
    api.poll_work(PollWorkRequest {
        worker_id: "other.run".to_string(),
        lease_id: "lease-3".to_string(),
        now: at(112),
        visibility_timeout: Duration::seconds(8),
    })
    .unwrap();

    let recovered = api
        .recover_expired_leases(RecoverExpiredLeasesRequest {
            queue: Some("worker.run".to_string()),
            now: at(120),
        })
        .expect("expired leases recover in one deterministic pass");

    assert_eq!(
        recovered
            .tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        vec!["task-1", "task-2"]
    );
    assert_eq!(
        api.store().queue_task("task-1").unwrap().status,
        QueueTaskStatus::Available
    );
    assert_eq!(
        api.store().queue_task("task-2").unwrap().status,
        QueueTaskStatus::Available
    );
    assert_eq!(
        api.store().queue_task("task-3").unwrap().status,
        QueueTaskStatus::Leased
    );
    assert_eq!(
        api.store()
            .queue_task("task-1")
            .unwrap()
            .events
            .last()
            .unwrap()
            .kind,
        QueueEventKind::LeaseExpired
    );

    let recovered = api
        .recover_expired_leases(RecoverExpiredLeasesRequest {
            queue: None,
            now: at(120),
        })
        .unwrap();
    assert_eq!(
        recovered
            .tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        vec!["task-3"]
    );
}

#[test]
fn bounded_polling_enforces_per_function_concurrency_at_queue_boundary() {
    let mut api = OpenViewControlApi::default();
    for (task_id, queue) in [
        ("task-1", "worker.run"),
        ("task-2", "worker.run"),
        ("task-3", "other.run"),
    ] {
        api.enqueue_task(EnqueueTaskRequest {
            task_id: task_id.to_string(),
            queue: queue.to_string(),
            payload: json!({}),
            visible_at: at(100),
        })
        .unwrap();
    }

    let first = api
        .poll_work_with_concurrency_limit(
            PollWorkRequest {
                worker_id: "worker.run".to_string(),
                lease_id: "lease-1".to_string(),
                now: at(110),
                visibility_timeout: Duration::seconds(30),
            },
            1,
        )
        .expect("first worker.run task can lease under cap");
    assert_eq!(
        first.task.as_ref().map(|task| task.id.as_str()),
        Some("task-1")
    );

    let blocked = api
        .poll_work_with_concurrency_limit(
            PollWorkRequest {
                worker_id: "worker.run".to_string(),
                lease_id: "lease-2".to_string(),
                now: at(111),
                visibility_timeout: Duration::seconds(30),
            },
            1,
        )
        .expect("cap returns an empty poll instead of over-leasing");
    assert!(blocked.task.is_none());
    assert!(blocked.lease.is_none());
    assert_eq!(
        api.store().queue_task("task-2").unwrap().status,
        QueueTaskStatus::Available
    );

    let other_queue = api
        .poll_work_with_concurrency_limit(
            PollWorkRequest {
                worker_id: "other.run".to_string(),
                lease_id: "lease-3".to_string(),
                now: at(112),
                visibility_timeout: Duration::seconds(30),
            },
            1,
        )
        .expect("concurrency cap is per function queue");
    assert_eq!(
        other_queue.task.as_ref().map(|task| task.id.as_str()),
        Some("task-3")
    );
}

#[test]
fn concurrency_snapshot_counts_active_leases_and_visible_available_tasks() {
    let mut api = OpenViewControlApi::default();
    for (task_id, queue, visible_at) in [
        ("active", "worker.run", at(100)),
        ("available", "worker.run", at(100)),
        ("delayed", "worker.run", at(150)),
        ("other", "other.run", at(100)),
    ] {
        api.enqueue_task(EnqueueTaskRequest {
            task_id: task_id.to_string(),
            queue: queue.to_string(),
            payload: json!({}),
            visible_at,
        })
        .unwrap();
    }

    api.poll_work(PollWorkRequest {
        worker_id: "worker.run".to_string(),
        lease_id: "lease-active".to_string(),
        now: at(110),
        visibility_timeout: Duration::seconds(30),
    })
    .unwrap();

    let snapshot = api
        .queue_concurrency_snapshot(QueueConcurrencySnapshotRequest {
            queue: "worker.run".to_string(),
            requested_max_concurrency: 2,
            now: at(120),
            policy_kind: QueueConcurrencyPolicyKind::LocallyEnforced,
        })
        .expect("snapshot reads queue state");

    assert_eq!(snapshot.queue, "worker.run");
    assert_eq!(snapshot.requested_max_concurrency, 2);
    assert_eq!(snapshot.active_lease_count, 1);
    assert_eq!(snapshot.available_task_count, 1);
    assert!(!snapshot.blocked_by_limit);
    assert_eq!(
        snapshot.policy_kind,
        QueueConcurrencyPolicyKind::LocallyEnforced
    );
    assert!(snapshot.locally_enforced);
}

#[test]
fn concurrency_snapshot_reports_local_limit_blocking_at_requested_max() {
    let mut api = OpenViewControlApi::default();
    for task_id in ["active", "available"] {
        api.enqueue_task(EnqueueTaskRequest {
            task_id: task_id.to_string(),
            queue: "worker.run".to_string(),
            payload: json!({}),
            visible_at: at(100),
        })
        .unwrap();
    }

    api.poll_work(PollWorkRequest {
        worker_id: "worker.run".to_string(),
        lease_id: "lease-active".to_string(),
        now: at(110),
        visibility_timeout: Duration::seconds(30),
    })
    .unwrap();

    let snapshot = api
        .queue_concurrency_snapshot(QueueConcurrencySnapshotRequest {
            queue: "worker.run".to_string(),
            requested_max_concurrency: 1,
            now: at(120),
            policy_kind: QueueConcurrencyPolicyKind::LocallyEnforced,
        })
        .expect("snapshot reads queue state");

    assert_eq!(snapshot.active_lease_count, 1);
    assert_eq!(snapshot.available_task_count, 1);
    assert!(snapshot.blocked_by_limit);
    assert!(snapshot.locally_enforced);
}

#[test]
fn concurrency_snapshot_represents_delegated_iii_queue_policy_without_local_blocking() {
    let mut api = OpenViewControlApi::default();
    for task_id in ["active", "available"] {
        api.enqueue_task(EnqueueTaskRequest {
            task_id: task_id.to_string(),
            queue: "worker.run".to_string(),
            payload: json!({}),
            visible_at: at(100),
        })
        .unwrap();
    }

    api.poll_work(PollWorkRequest {
        worker_id: "worker.run".to_string(),
        lease_id: "lease-active".to_string(),
        now: at(110),
        visibility_timeout: Duration::seconds(30),
    })
    .unwrap();

    let snapshot = api
        .queue_concurrency_snapshot(QueueConcurrencySnapshotRequest {
            queue: "worker.run".to_string(),
            requested_max_concurrency: 1,
            now: at(120),
            policy_kind: QueueConcurrencyPolicyKind::DelegatedToIiiQueue,
        })
        .expect("snapshot reads delegated queue state");

    assert_eq!(snapshot.active_lease_count, 1);
    assert_eq!(snapshot.available_task_count, 1);
    assert_eq!(
        snapshot.policy_kind,
        QueueConcurrencyPolicyKind::DelegatedToIiiQueue
    );
    assert!(!snapshot.blocked_by_limit);
    assert!(!snapshot.locally_enforced);
}

#[test]
fn queue_contract_serializes_with_snake_case_status_and_stable_events() {
    let mut task = QueueTask::new("task-1", "worker.run", json!({"goal": "ship"}), at(100));
    task.lease("lease-1", "worker-a", at(110), Duration::seconds(30))
        .unwrap();
    task.ack("lease-1", at(120)).unwrap();

    let value = serde_json::to_value(&task).expect("queue task serializes");

    assert_eq!(value["status"], "completed");
    assert_eq!(value["retry_count"], 0);
    assert_eq!(value["visible_at"], "1970-01-01T00:02:20Z");
    assert_eq!(value["events"][0]["kind"], "enqueued");
    assert_eq!(value["events"][1]["kind"], "leased");
    assert_eq!(value["events"][2]["kind"], "completed");
}
