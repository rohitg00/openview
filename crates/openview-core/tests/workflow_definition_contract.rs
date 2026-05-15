use openview_core::{OpenViewError, WorkflowDefinition, WorkflowNode, WorkflowNodeKind};
use serde_json::json;

#[test]
fn workflow_definition_serializes_fanout_join_and_approval_nodes() {
    let workflow = WorkflowDefinition::new("wf.review_pipeline", "1.0.0", "Review pipeline")
        .input_schema(json!({
            "type": "object",
            "required": ["pr_number"],
            "properties": {"pr_number": {"type": "integer"}}
        }))
        .node(WorkflowNode::action("fetch_pr", "github::fetch_pr"))
        .node(WorkflowNode::fanout("fanout_checks", "checks::run_all").depends_on("fetch_pr"))
        .node(WorkflowNode::join("join_checks", "checks::join").depends_on("fanout_checks"))
        .node(
            WorkflowNode::approval("approve_merge", "approval::request").depends_on("join_checks"),
        );

    let round_trip: WorkflowDefinition =
        serde_json::from_value(serde_json::to_value(&workflow).unwrap()).unwrap();

    assert_eq!(round_trip.id, "wf.review_pipeline");
    assert_eq!(round_trip.version, "1.0.0");
    assert_eq!(round_trip.name, "Review pipeline");
    assert_eq!(round_trip.input_schema["required"], json!(["pr_number"]));
    assert_eq!(round_trip.nodes[0].kind, WorkflowNodeKind::Action);
    assert_eq!(round_trip.nodes[1].kind, WorkflowNodeKind::Fanout);
    assert_eq!(round_trip.nodes[2].kind, WorkflowNodeKind::Join);
    assert_eq!(round_trip.nodes[3].kind, WorkflowNodeKind::Approval);
    assert_eq!(
        round_trip.nodes[3].dependencies,
        vec!["join_checks".to_string()]
    );

    let serialized = serde_json::to_value(&round_trip).unwrap();
    assert_eq!(serialized["nodes"][1]["kind"], "fanout");
    assert_eq!(
        serialized["nodes"][3]["dependencies"],
        json!(["join_checks"])
    );
}

#[test]
fn execution_plan_is_a_deterministic_topological_order_not_insertion_order() {
    let workflow = WorkflowDefinition::new("wf.out_of_order", "1.0.0", "Out of order")
        .node(
            WorkflowNode::join("finalize", "pipeline::finalize")
                .depends_on("fast")
                .depends_on("slow"),
        )
        .node(WorkflowNode::action("slow", "pipeline::slow"))
        .node(WorkflowNode::fanout("branch", "pipeline::fanout").depends_on("slow"))
        .node(WorkflowNode::action("fast", "pipeline::fast"))
        .node(WorkflowNode::join("after_branch", "pipeline::join").depends_on("branch"));

    let first_plan = workflow.execution_plan().unwrap();
    let second_plan = workflow.execution_plan().unwrap();

    assert_eq!(first_plan, second_plan);
    assert_eq!(
        first_plan,
        vec![
            "fast".to_string(),
            "slow".to_string(),
            "branch".to_string(),
            "after_branch".to_string(),
            "finalize".to_string(),
        ]
    );
}

#[test]
fn validation_rejects_unknown_dependencies_and_cycles() {
    let unknown = WorkflowDefinition::new("wf.unknown", "1.0.0", "Unknown dependency")
        .node(WorkflowNode::action("start", "pipeline::start"))
        .node(WorkflowNode::join("finish", "pipeline::finish").depends_on("missing"));

    assert_eq!(
        unknown.validate(),
        Err(OpenViewError::UnknownWorkflowDependency {
            workflow_id: "wf.unknown".to_string(),
            node_id: "finish".to_string(),
            dependency_id: "missing".to_string(),
        })
    );

    let cycle = WorkflowDefinition::new("wf.cycle", "1.0.0", "Cycle")
        .node(WorkflowNode::action("a", "pipeline::a").depends_on("c"))
        .node(WorkflowNode::action("b", "pipeline::b").depends_on("a"))
        .node(WorkflowNode::action("c", "pipeline::c").depends_on("b"));

    assert_eq!(
        cycle.execution_plan(),
        Err(OpenViewError::WorkflowCycle {
            workflow_id: "wf.cycle".to_string(),
        })
    );
}
