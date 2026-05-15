use openview_worker::{
    ReadinessProbe, RedactionRule, RestartPolicy, StopPolicy, WorkerProcessRequest,
    WorkerProcessStdio, WorkerStdioStream,
};
use serde_json::json;

#[test]
fn worker_process_request_builds_secret_free_invocation_contract() {
    let request = WorkerProcessRequest::new("agent-memory", "agentmemory")
        .arg("serve")
        .arg("--port")
        .arg("3000")
        .working_directory("/repo/agentmemory")
        .env_key("OPENAI_API_KEY")
        .env_key("DATABASE_URL")
        .process_group(true)
        .readiness_probe(ReadinessProbe::tcp("127.0.0.1", 3000).initial_delay_ms(250))
        .restart_policy(RestartPolicy::on_failure(3).backoff_ms(500))
        .stop_policy(StopPolicy::term_then_kill(1_500));

    let invocation = request.invocation();

    assert_eq!(invocation.worker_id, "agent-memory");
    assert_eq!(invocation.binary, "agentmemory");
    assert_eq!(invocation.argv, vec!["serve", "--port", "3000"]);
    assert_eq!(
        invocation.working_directory.as_deref(),
        Some("/repo/agentmemory")
    );
    assert_eq!(invocation.env_keys, vec!["OPENAI_API_KEY", "DATABASE_URL"]);
    assert!(invocation.create_process_group);
    assert_eq!(
        invocation.readiness_probe,
        Some(ReadinessProbe::tcp("127.0.0.1", 3000).initial_delay_ms(250))
    );
    assert_eq!(
        invocation.restart_policy,
        RestartPolicy::on_failure(3).backoff_ms(500)
    );
    assert_eq!(invocation.stop_policy, StopPolicy::term_then_kill(1_500));

    let value = serde_json::to_value(&invocation).expect("invocation serializes");
    assert!(
        value.get("env").is_none(),
        "env values must never serialize"
    );
    assert!(
        value.get("environment").is_none(),
        "env maps must never serialize"
    );
}

#[test]
fn stdout_and_stderr_events_are_sanitized_with_deterministic_redaction_rules() {
    let rules = vec![
        RedactionRule::literal("sk-live-123", "[api-key]"),
        RedactionRule::env_assignment("DATABASE_URL"),
    ];

    let stdout = WorkerProcessStdio::event_payload(
        "agent-memory",
        WorkerStdioStream::Stdout,
        42,
        "ready with token sk-live-123",
        &rules,
    );
    let stderr = WorkerProcessStdio::event_payload(
        "agent-memory",
        WorkerStdioStream::Stderr,
        43,
        "DATABASE_URL=postgres://user:pass@localhost/db failed",
        &rules,
    );

    assert_eq!(
        stdout,
        json!({
            "worker_id": "agent-memory",
            "stream": "stdout",
            "sequence": 42,
            "text": "ready with token [api-key]",
            "redacted": true
        })
    );
    assert_eq!(
        stderr,
        json!({
            "worker_id": "agent-memory",
            "stream": "stderr",
            "sequence": 43,
            "text": "DATABASE_URL=[redacted] failed",
            "redacted": true
        })
    );
}

#[test]
fn defaults_are_safe_and_deterministic_for_process_launches() {
    let invocation = WorkerProcessRequest::new("approval-gate", "approval-worker").invocation();

    assert_eq!(invocation.argv, Vec::<String>::new());
    assert_eq!(invocation.env_keys, Vec::<String>::new());
    assert_eq!(invocation.working_directory, None);
    assert!(!invocation.create_process_group);
    assert_eq!(invocation.readiness_probe, None);
    assert_eq!(invocation.restart_policy, RestartPolicy::Never);
    assert_eq!(invocation.stop_policy, StopPolicy::term_then_kill(5_000));
}
