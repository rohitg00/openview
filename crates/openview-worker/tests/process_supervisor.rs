use openview_worker::{
    ReadinessProbe, RedactionRule, RestartPolicy, StopPolicy,
    WorkerProcessQueueLeaseHeartbeatPolicy, WorkerProcessRequest, WorkerProcessState,
    WorkerProcessSupervisor, WorkerProcessSupervisorEvent, WorkerStdioStream,
    WORKER_PROCESS_QUEUE_LEASE_HEARTBEAT_CONTROL_API_METHOD,
};
use serde_json::json;

fn invocation() -> openview_worker::WorkerProcessInvocation {
    WorkerProcessRequest::new("agent-memory", "agentmemory")
        .arg("serve")
        .readiness_probe(ReadinessProbe::tcp("127.0.0.1", 3000).initial_delay_ms(100))
        .restart_policy(RestartPolicy::on_failure(2).backoff_ms(250))
        .stop_policy(StopPolicy::term_then_kill(1_500))
        .invocation()
}

fn heartbeat_invocation() -> openview_worker::WorkerProcessInvocation {
    WorkerProcessRequest::new("agent-memory", "agentmemory")
        .arg("serve")
        .readiness_probe(ReadinessProbe::tcp("127.0.0.1", 3000).initial_delay_ms(100))
        .queue_lease_heartbeat(
            WorkerProcessQueueLeaseHeartbeatPolicy::new("agent.queue", 60_000)
                .metadata_keys(["phase", "pid", "detail", "nested", "token", "api_key"]),
        )
        .invocation()
}

fn heartbeat_request(
    queue: impl Into<String>,
    visibility_timeout_secs: u64,
    metadata: serde_json::Value,
) -> openview_core::HeartbeatWorkerLeaseRequest {
    openview_core::HeartbeatWorkerLeaseRequest {
        worker_id: "agent-memory".to_string(),
        queue: queue.into(),
        heartbeat_at: "2026-05-15T12:00:00Z".parse().unwrap(),
        visibility_timeout: serde_json::from_value(json!([visibility_timeout_secs, 0])).unwrap(),
        metadata: Some(metadata),
    }
}

#[test]
fn start_requested_transitions_to_ready_with_probe_metadata() {
    let mut supervisor = WorkerProcessSupervisor::new(invocation());

    let start = supervisor.request_start();
    let ready = supervisor.mark_ready("tcp probe connected");

    assert_eq!(supervisor.state(), WorkerProcessState::Ready);
    assert_eq!(
        supervisor.events(),
        &[
            WorkerProcessSupervisorEvent::StartRequested {
                sequence: 1,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                invocation: invocation(),
            },
            WorkerProcessSupervisorEvent::Ready {
                sequence: 2,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                readiness_probe: Some(ReadinessProbe::tcp("127.0.0.1", 3000).initial_delay_ms(100)),
                message: "tcp probe connected".to_string(),
            },
        ]
    );
    assert_eq!(start.sequence(), 1);
    assert_eq!(ready.sequence(), 2);
}

#[test]
fn stdout_and_stderr_log_events_are_sanitized_and_sequenced() {
    let mut supervisor = WorkerProcessSupervisor::new(invocation()).with_redaction_rules(vec![
        RedactionRule::literal("sk-live-123", "[api-key]"),
        RedactionRule::env_assignment("DATABASE_URL"),
    ]);

    supervisor.request_start();
    supervisor.record_stdout("ready sk-live-123");
    supervisor.record_stderr("DATABASE_URL=postgres://user:pass@localhost/db failed");

    assert_eq!(
        supervisor.events(),
        &[
            WorkerProcessSupervisorEvent::StartRequested {
                sequence: 1,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                invocation: invocation(),
            },
            WorkerProcessSupervisorEvent::Log {
                sequence: 2,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                stream: WorkerStdioStream::Stdout,
                payload: json!({
                    "worker_id": "agent-memory",
                    "stream": "stdout",
                    "sequence": 2,
                    "text": "ready [api-key]",
                    "redacted": true
                }),
            },
            WorkerProcessSupervisorEvent::Log {
                sequence: 3,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                stream: WorkerStdioStream::Stderr,
                payload: json!({
                    "worker_id": "agent-memory",
                    "stream": "stderr",
                    "sequence": 3,
                    "text": "DATABASE_URL=[redacted] failed",
                    "redacted": true
                }),
            },
        ]
    );
}

#[test]
fn queue_lease_heartbeat_ticks_emit_sequenced_control_api_events() {
    let mut supervisor = WorkerProcessSupervisor::new(heartbeat_invocation());

    supervisor.request_start();
    supervisor.mark_ready("tcp probe connected");
    let first = supervisor
        .record_queue_lease_heartbeat(&heartbeat_request(
            "agent.queue",
            60,
            json!({"phase": "running", "pid": 4242}),
        ))
        .expect("ready worker records matching queue heartbeat");
    let second = supervisor
        .record_queue_lease_heartbeat(&heartbeat_request(
            "agent.queue",
            60,
            json!({"phase": "checkpoint", "pid": 4242}),
        ))
        .expect("subsequent matching heartbeat is recorded");

    assert_eq!(first.sequence(), 3);
    assert_eq!(second.sequence(), 4);
    assert_eq!(
        &supervisor.events()[2..],
        &[
            WorkerProcessSupervisorEvent::Heartbeat {
                sequence: 3,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                queue: "agent.queue".to_string(),
                visibility_timeout_ms: 60_000,
                metadata: json!({"phase": "running", "pid": 4242}),
                control_api_method: WORKER_PROCESS_QUEUE_LEASE_HEARTBEAT_CONTROL_API_METHOD
                    .to_string(),
            },
            WorkerProcessSupervisorEvent::Heartbeat {
                sequence: 4,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                queue: "agent.queue".to_string(),
                visibility_timeout_ms: 60_000,
                metadata: json!({"phase": "checkpoint", "pid": 4242}),
                control_api_method: WORKER_PROCESS_QUEUE_LEASE_HEARTBEAT_CONTROL_API_METHOD
                    .to_string(),
            },
        ]
    );
}

#[test]
fn queue_lease_heartbeat_records_policy_and_sanitized_metadata_only() {
    let mut supervisor =
        WorkerProcessSupervisor::new(heartbeat_invocation()).with_redaction_rules(vec![
            RedactionRule::literal("sk-live-123", "[api-key]"),
            RedactionRule::env_assignment("DATABASE_URL"),
        ]);

    supervisor.request_start();
    supervisor.mark_ready("ready");
    supervisor
        .record_queue_lease_heartbeat(&heartbeat_request(
            "agent.queue",
            60,
            json!({
                "phase": "running",
                "pid": 4242,
                "detail": "using sk-live-123 DATABASE_URL=postgres://user:pass@localhost/db",
                "nested": {
                    "safe": "ok",
                    "password": "never-emit"
                },
                "token": "secret-token",
                "api_key": "sk-live-123",
                "ignored": "not-whitelisted"
            }),
        ))
        .expect("heartbeat is recorded");

    assert_eq!(
        supervisor.events().last().unwrap(),
        &WorkerProcessSupervisorEvent::Heartbeat {
            sequence: 3,
            worker_id: "agent-memory".to_string(),
            attempt: 1,
            queue: "agent.queue".to_string(),
            visibility_timeout_ms: 60_000,
            metadata: json!({
                "phase": "running",
                "pid": 4242,
                "detail": "using [api-key] DATABASE_URL=[redacted]",
                "nested": {
                    "safe": "ok"
                }
            }),
            control_api_method: WORKER_PROCESS_QUEUE_LEASE_HEARTBEAT_CONTROL_API_METHOD.to_string(),
        }
    );
}

#[test]
fn queue_lease_heartbeat_does_not_emit_when_not_ready_or_policy_mismatch() {
    let mut supervisor = WorkerProcessSupervisor::new(heartbeat_invocation());

    assert!(supervisor
        .record_queue_lease_heartbeat(&heartbeat_request("agent.queue", 60, json!({})))
        .is_none());
    supervisor.request_start();
    supervisor.mark_ready("ready");
    assert!(supervisor
        .record_queue_lease_heartbeat(&heartbeat_request("other.queue", 60, json!({})))
        .is_none());
    assert!(supervisor
        .record_queue_lease_heartbeat(&heartbeat_request("agent.queue", 30, json!({})))
        .is_none());
    assert_eq!(supervisor.events().len(), 2);
}

#[test]
fn queue_lease_heartbeat_events_do_not_leak_secrets() {
    let mut supervisor =
        WorkerProcessSupervisor::new(heartbeat_invocation()).with_redaction_rules(vec![
            RedactionRule::literal("sk-live-123", "[api-key]"),
            RedactionRule::env_assignment("DATABASE_URL"),
        ]);

    supervisor.request_start();
    supervisor.mark_ready("ready");
    supervisor
        .record_queue_lease_heartbeat(&heartbeat_request(
            "agent.queue",
            60,
            json!({
                "detail": "sk-live-123 DATABASE_URL=postgres://user:pass@localhost/db",
                "token": "secret-token",
                "nested": {
                    "password": "super-secret",
                    "safe": "visible"
                }
            }),
        ))
        .expect("heartbeat is recorded");

    let event_json = serde_json::to_string(supervisor.events().last().unwrap()).unwrap();
    assert!(!event_json.contains("sk-live-123"));
    assert!(!event_json.contains("postgres://user:pass@localhost/db"));
    assert!(!event_json.contains("secret-token"));
    assert!(!event_json.contains("super-secret"));
    assert!(event_json.contains("[api-key]"));
    assert!(event_json.contains("[redacted]"));
}

#[test]
fn failures_restart_until_max_restarts_then_enter_failed_terminal_state() {
    let mut supervisor = WorkerProcessSupervisor::new(invocation());

    supervisor.request_start();
    supervisor.record_exit_failure(1, "first crash");
    supervisor.record_exit_failure(2, "second crash");
    supervisor.record_exit_failure(3, "third crash");

    assert_eq!(supervisor.state(), WorkerProcessState::Failed);
    assert_eq!(supervisor.restart_count(), 2);
    assert_eq!(
        supervisor.events(),
        &[
            WorkerProcessSupervisorEvent::StartRequested {
                sequence: 1,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                invocation: invocation(),
            },
            WorkerProcessSupervisorEvent::ExitFailure {
                sequence: 2,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                exit_code: 1,
                message: "first crash".to_string(),
            },
            WorkerProcessSupervisorEvent::RestartScheduled {
                sequence: 3,
                worker_id: "agent-memory".to_string(),
                next_attempt: 2,
                restart_count: 1,
                backoff_ms: 250,
            },
            WorkerProcessSupervisorEvent::StartRequested {
                sequence: 4,
                worker_id: "agent-memory".to_string(),
                attempt: 2,
                invocation: invocation(),
            },
            WorkerProcessSupervisorEvent::ExitFailure {
                sequence: 5,
                worker_id: "agent-memory".to_string(),
                attempt: 2,
                exit_code: 2,
                message: "second crash".to_string(),
            },
            WorkerProcessSupervisorEvent::RestartScheduled {
                sequence: 6,
                worker_id: "agent-memory".to_string(),
                next_attempt: 3,
                restart_count: 2,
                backoff_ms: 250,
            },
            WorkerProcessSupervisorEvent::StartRequested {
                sequence: 7,
                worker_id: "agent-memory".to_string(),
                attempt: 3,
                invocation: invocation(),
            },
            WorkerProcessSupervisorEvent::ExitFailure {
                sequence: 8,
                worker_id: "agent-memory".to_string(),
                attempt: 3,
                exit_code: 3,
                message: "third crash".to_string(),
            },
            WorkerProcessSupervisorEvent::Failed {
                sequence: 9,
                worker_id: "agent-memory".to_string(),
                attempt: 3,
                reason: "max restarts exceeded after 2 restarts".to_string(),
            },
        ]
    );
}

#[test]
fn graceful_stop_records_stop_policy_metadata_and_stopped_terminal_state() {
    let mut supervisor = WorkerProcessSupervisor::new(invocation());

    supervisor.request_start();
    supervisor.mark_ready("ready");
    supervisor.request_graceful_stop("operator requested shutdown");
    supervisor.mark_stopped(0, "process exited cleanly");

    assert_eq!(supervisor.state(), WorkerProcessState::Stopped);
    assert_eq!(
        supervisor.events(),
        &[
            WorkerProcessSupervisorEvent::StartRequested {
                sequence: 1,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                invocation: invocation(),
            },
            WorkerProcessSupervisorEvent::Ready {
                sequence: 2,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                readiness_probe: Some(ReadinessProbe::tcp("127.0.0.1", 3000).initial_delay_ms(100)),
                message: "ready".to_string(),
            },
            WorkerProcessSupervisorEvent::GracefulStopRequested {
                sequence: 3,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                stop_policy: StopPolicy::term_then_kill(1_500),
                reason: "operator requested shutdown".to_string(),
            },
            WorkerProcessSupervisorEvent::Stopped {
                sequence: 4,
                worker_id: "agent-memory".to_string(),
                attempt: 1,
                exit_code: 0,
                message: "process exited cleanly".to_string(),
            },
        ]
    );
}
