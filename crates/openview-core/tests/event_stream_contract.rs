use openview_core::{
    CapabilityRisk, EventCursor, FunctionSpec, OpenViewError, OpenViewRuntime, RunPhase,
    SandboxProfile, WorkerManifest,
};
use uuid::Uuid;

fn runtime_with_approval_worker() -> OpenViewRuntime {
    let mut runtime = OpenViewRuntime::default();
    runtime
        .register_worker(
            WorkerManifest::new("shell.sandbox", "0.1.0")
                .function(
                    FunctionSpec::new("shell::run", CapabilityRisk::Exec).approval_required(true),
                )
                .sandbox(SandboxProfile::locked_down()),
        )
        .unwrap();
    runtime
}

fn completed_approval_run(runtime: &mut OpenViewRuntime) -> Uuid {
    let run = runtime
        .start_run("replay approval trace", ["shell.sandbox"])
        .expect("run starts");
    assert_eq!(run.phase, RunPhase::WaitingForApproval);
    let resumed = runtime
        .approve(run.id, run.approvals[0].id, "approved for replay")
        .expect("approval resumes run");
    assert_eq!(resumed.phase, RunPhase::Completed);
    run.id
}

#[test]
fn approval_flow_events_are_replayable_in_sequence_by_cursor() {
    let mut runtime = runtime_with_approval_worker();
    let run_id = completed_approval_run(&mut runtime);

    let mut cursor = EventCursor {
        run_id,
        after_sequence: 0,
        limit: 1,
    };
    let mut replayed = Vec::new();
    loop {
        let page = runtime
            .events_since(cursor.run_id, cursor.after_sequence, cursor.limit)
            .expect("events page is available");
        assert_eq!(page.run_id, run_id);
        replayed.extend(
            page.events
                .iter()
                .map(|event| (event.sequence, event.kind.clone())),
        );
        cursor.after_sequence = page.next_after_sequence;
        if !page.has_more {
            break;
        }
    }

    assert_eq!(
        replayed,
        vec![
            (1, "run.started".to_string()),
            (2, "worker.started".to_string()),
            (3, "approval.requested".to_string()),
            (4, "approval.approved".to_string()),
            (5, "run.completed".to_string()),
        ]
    );
    assert_eq!(cursor.after_sequence, 5);
}

#[test]
fn event_stream_pagination_respects_limit_and_has_more() {
    let mut runtime = runtime_with_approval_worker();
    let run_id = completed_approval_run(&mut runtime);

    let first = runtime
        .events_since(run_id, 0, 2)
        .expect("first page is available");
    assert_eq!(first.events.len(), 2);
    assert_eq!(first.next_after_sequence, 2);
    assert!(first.has_more);

    let second = runtime
        .events_since(run_id, first.next_after_sequence, 2)
        .expect("second page is available");
    assert_eq!(second.events.len(), 2);
    assert_eq!(second.events[0].sequence, 3);
    assert_eq!(second.next_after_sequence, 4);
    assert!(second.has_more);

    let third = runtime
        .events_since(run_id, second.next_after_sequence, 2)
        .expect("third page is available");
    assert_eq!(third.events.len(), 1);
    assert_eq!(third.events[0].sequence, 5);
    assert_eq!(third.next_after_sequence, 5);
    assert!(!third.has_more);
}

#[test]
fn event_stream_empty_reads_keep_stable_cursor() {
    let mut runtime = runtime_with_approval_worker();
    let run_id = completed_approval_run(&mut runtime);

    let empty = runtime
        .events_since(run_id, 5, 10)
        .expect("empty page is available");

    assert_eq!(empty.run_id, run_id);
    assert!(empty.events.is_empty());
    assert_eq!(empty.next_after_sequence, 5);
    assert!(!empty.has_more);

    let zero_limit = runtime
        .events_since(run_id, 0, 0)
        .expect("zero limit page is available");
    assert!(zero_limit.events.is_empty());
    assert_eq!(zero_limit.next_after_sequence, 0);
    assert!(zero_limit.has_more);
}

#[test]
fn event_stream_unknown_run_errors() {
    let runtime = OpenViewRuntime::default();
    let missing_run_id = Uuid::new_v4();

    let error = runtime
        .events_since(missing_run_id, 0, 10)
        .expect_err("unknown run errors");

    assert!(matches!(error, OpenViewError::RunNotFound(id) if id == missing_run_id));
}
