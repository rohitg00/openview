use openview_core::{
    CheckRunStatus, DiffReviewComment, MergeReadiness, PullRequestMetadata, ReviewWorkflow,
    TodoBlocker,
};

#[test]
fn review_workflow_blocks_merge_on_unresolved_comments_failing_checks_and_open_todos() {
    let workflow = ReviewWorkflow::new(PullRequestMetadata::new(
        42,
        "feature/review-contracts",
        "main",
    ))
    .review_comment(
        DiffReviewComment::new("crates/openview-core/src/lib.rs", 128, "missing edge case")
            .author("reviewer")
            .unresolved(),
    )
    .check_run(CheckRunStatus::failed("cargo test -p openview-core"))
    .check_run(CheckRunStatus::passed("cargo fmt --all --check"))
    .todo_blocker(TodoBlocker::new("docs", "document readiness contract"));

    let readiness = workflow.merge_readiness();

    assert_eq!(readiness, MergeReadiness::Blocked);
    assert_eq!(workflow.pr.number, 42);
    assert_eq!(workflow.pr.head_branch, "feature/review-contracts");
    assert_eq!(workflow.unresolved_review_comments().count(), 1);
    assert_eq!(workflow.failing_checks().count(), 1);
    assert_eq!(workflow.open_todo_blockers().count(), 1);
    assert_eq!(
        workflow.readiness_summary(),
        "blocked: 1 unresolved review comment, 1 failing check, 1 open todo blocker"
    );
}

#[test]
fn review_workflow_is_merge_ready_when_comments_checks_and_todos_are_resolved() {
    let workflow = ReviewWorkflow::new(PullRequestMetadata::new(7, "fix/tests", "main"))
        .review_comment(
            DiffReviewComment::new("crates/openview-core/src/lib.rs", 88, "nit fixed")
                .author("reviewer")
                .resolved(),
        )
        .check_run(CheckRunStatus::passed("cargo test -p openview-core"))
        .todo_blocker(TodoBlocker::new("review", "human review complete").resolved());

    assert_eq!(workflow.merge_readiness(), MergeReadiness::Ready);
    assert_eq!(
        workflow.readiness_summary(),
        "ready: all review workflow contracts satisfied"
    );
    assert_eq!(workflow.unresolved_review_comments().count(), 0);
    assert_eq!(workflow.failing_checks().count(), 0);
    assert_eq!(workflow.open_todo_blockers().count(), 0);
}

#[test]
fn review_workflow_serializes_as_deterministic_snake_case_contract() {
    let workflow = ReviewWorkflow::new(
        PullRequestMetadata::new(9, "feat/contracts", "main")
            .title("Add review workflow contracts")
            .author("agent"),
    )
    .review_comment(
        DiffReviewComment::new("crates/openview-core/src/lib.rs", 144, "looks good")
            .author("reviewer")
            .resolved(),
    )
    .check_run(CheckRunStatus::pending("clippy"))
    .todo_blocker(TodoBlocker::new("merge", "wait for clippy"));

    let json = serde_json::to_value(&workflow).expect("workflow serializes");

    assert_eq!(json["pr"]["number"], 9);
    assert_eq!(json["pr"]["head_branch"], "feat/contracts");
    assert_eq!(
        json["review_comments"][0]["path"],
        "crates/openview-core/src/lib.rs"
    );
    assert_eq!(json["review_comments"][0]["resolved"], true);
    assert_eq!(json["checks"][0]["conclusion"], "pending");
    assert_eq!(json["todo_blockers"][0]["resolved"], false);
}
