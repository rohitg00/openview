use openview_core::{BackendTarget, CapabilityRisk, FunctionVisibility, ResourceKind};
use openview_worker::{
    approval_list_pending_invocation, approval_resolve_invocation,
    hook_fanout_publish_collect_invocation, iii_database_execute_invocation,
    iii_database_query_invocation, iii_database_transaction_invocation,
    iii_durable_publish_invocation, iii_named_queue_enqueue_invocation,
    openview_iii_backend_adapter_manifest, openview_queue_recovery_scan_plan,
    openview_queue_schema_bootstrap_plan, openview_queue_task_claim_lease_plan,
    openview_queue_task_complete_plan, openview_queue_task_fail_dead_letter_plan,
    openview_queue_task_fail_retry_plan, openview_queue_task_heartbeat_lease_plan,
    openview_queue_task_spawn_plan, run_start_and_wait_invocation, run_start_invocation,
    session_tree_append_invocation, session_tree_create_invocation,
    session_tree_messages_invocation, shell_sandbox_exec_invocation, stream_list_invocation,
    stream_set_invocation, IiiTriggerInvocation, IiiTriggerPlan, OpenViewQueueRetryPlanRequest,
    OPENVIEW_QUEUE_INDEX_NAMES, OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION,
    OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID, OPENVIEW_QUEUE_SCHEMA_VERSION, OPENVIEW_QUEUE_TABLE_NAMES,
};
use serde_json::json;

fn assert_serialized_without_secret_material(value: &serde_json::Value) {
    let serialized = serde_json::to_string(value).expect("value serializes");
    for forbidden in [
        "sk_live_should_not_serialize",
        "super-secret-token",
        "api_key",
        "password",
        "credential",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "serialized contract leaked {forbidden}: {serialized}"
        );
    }
}

#[test]
fn serializes_iii_trigger_invocations_for_database_stream_hooks_approval_runs_sessions_and_shell() {
    let calls = vec![
        iii_database_query_invocation(
            "primary",
            "SELECT id FROM tasks WHERE status = ?",
            vec![json!("open")],
        ),
        iii_database_execute_invocation(
            "primary",
            "UPDATE tasks SET status = ? WHERE id = ?",
            vec![json!("done"), json!(7)],
            vec!["id", "status"],
        ),
        iii_database_transaction_invocation(
            "primary",
            vec![
                json!({"sql": "INSERT INTO audit(message) VALUES (?)", "params": ["started"]}),
                json!({"sql": "SELECT changes() AS count", "params": []}),
            ],
        ),
        stream_set_invocation(
            "agent::events",
            "session-1",
            "session-00000001",
            json!({"type": "turn_start"}),
        ),
        stream_list_invocation("agent::events", "session-1"),
        iii_named_queue_enqueue_invocation(
            "openview::run_task",
            "agent-runs",
            json!({"task_id": "task-1"}),
        ),
        iii_durable_publish_invocation("openview::cancel", json!({"task_id": "task-1"})),
        hook_fanout_publish_collect_invocation(
            "agent::before_function_call",
            json!({"function_call": {"id": "call_1"}}),
            "first_block_wins",
            5_000,
        ),
        approval_resolve_invocation("call_1", "allow", Some("operator approved")),
        approval_list_pending_invocation("session-1"),
        run_start_invocation(
            "session-1",
            "anthropic",
            "claude-sonnet-4-5",
            vec![json!({"role": "user", "content": [{"type": "text", "text": "hello"}]})],
        ),
        run_start_and_wait_invocation(
            "session-2",
            "anthropic",
            "claude-sonnet-4-5",
            vec![json!({"role": "user", "content": [{"type": "text", "text": "ship it"}]})],
            60_000,
        ),
        session_tree_create_invocation("OpenView run"),
        session_tree_append_invocation(
            "session-tree-1",
            json!({"role": "assistant", "content": [{"type": "text", "text": "done"}], "timestamp": 0}),
            Some("root"),
        ),
        session_tree_messages_invocation("session-tree-1", Some(50)),
        shell_sandbox_exec_invocation(
            "sandbox-1",
            "cargo",
            vec!["test", "-p", "openview-worker"],
            Some(120_000),
        ),
    ];

    let serialized = serde_json::to_value(&calls).expect("iii trigger invocations serialize");

    assert_eq!(serialized[0]["function_id"], "iii-database::query");
    assert_eq!(serialized[0]["payload"]["params"], json!(["open"]));
    assert_eq!(serialized[1]["function_id"], "iii-database::execute");
    assert_eq!(
        serialized[1]["payload"]["returning"],
        json!(["id", "status"])
    );
    assert_eq!(serialized[2]["function_id"], "iii-database::transaction");
    assert_eq!(
        serialized[2]["payload"]["statements"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(serialized[3]["function_id"], "stream::set");
    assert_eq!(serialized[3]["payload"]["stream_name"], "agent::events");
    assert_eq!(serialized[3]["payload"]["group_id"], "session-1");
    assert_eq!(serialized[3]["payload"]["data"]["type"], "turn_start");
    assert_eq!(serialized[4]["function_id"], "stream::list");
    assert_eq!(serialized[4]["payload"]["group_id"], "session-1");
    assert_eq!(serialized[5]["function_id"], "openview::run_task");
    assert_eq!(
        serialized[5]["action"],
        json!({"kind": "enqueue", "queue": "agent-runs"})
    );
    assert_eq!(serialized[6]["function_id"], "iii::durable::publish");
    assert_eq!(serialized[6]["payload"]["topic"], "openview::cancel");
    assert_eq!(serialized[7]["function_id"], "hook-fanout::publish_collect");
    assert_eq!(serialized[7]["timeout_ms"], 5_000);
    assert_eq!(serialized[8]["function_id"], "approval::resolve");
    assert_eq!(serialized[8]["payload"]["decision"], "allow");
    assert_eq!(serialized[9]["function_id"], "approval::list_pending");
    assert_eq!(serialized[10]["function_id"], "run::start");
    assert_eq!(serialized[11]["function_id"], "run::start_and_wait");
    assert_eq!(serialized[11]["payload"]["timeout_ms"], 60_000);
    assert_eq!(serialized[12]["function_id"], "session-tree::create");
    assert_eq!(serialized[13]["function_id"], "session-tree::append");
    assert_eq!(serialized[13]["payload"]["parent_id"], "root");
    assert_eq!(serialized[14]["function_id"], "session-tree::messages");
    assert_eq!(serialized[15]["function_id"], "shell::exec");
    assert_eq!(
        serialized[15]["payload"]["target"],
        json!({"kind": "sandbox", "sandbox_id": "sandbox-1"})
    );
}

#[test]
fn queue_spawn_claim_and_heartbeat_plans_bind_database_leases_and_queue_dispatch() {
    let spawn = openview_queue_task_spawn_plan(
        "openview",
        "agent-runs",
        "task-1",
        "research-brief",
        "idem-task-1",
        "blob://openview/tasks/task-1/input",
        "sha256:input",
    );
    let claim = openview_queue_task_claim_lease_plan(
        "openview",
        "agent-runs",
        "worker-a",
        "lease-1",
        "2026-05-15T10:00:30Z",
        1,
    );
    let heartbeat = openview_queue_task_heartbeat_lease_plan(
        "openview",
        "task-1",
        "lease-1",
        "worker-a",
        "2026-05-15T10:01:00Z",
    );

    let spawn_json = serde_json::to_value(&spawn).expect("spawn plan serializes");
    let claim_json = serde_json::to_value(&claim).expect("claim plan serializes");
    let heartbeat_json = serde_json::to_value(&heartbeat).expect("heartbeat plan serializes");

    assert_eq!(
        spawn_json["required_workers"],
        json!(["iii-database", "iii-queue"])
    );
    assert_eq!(spawn_json["resources"], json!(["database", "event_stream"]));
    assert_eq!(
        spawn_json["calls"][0]["function_id"],
        "iii-database::transaction"
    );
    assert_eq!(
        spawn_json["calls"][1]["function_id"],
        "openview.queue::dispatch"
    );
    assert_eq!(
        spawn_json["calls"][1]["action"],
        json!({"kind": "enqueue", "queue": "agent-runs"})
    );
    assert_eq!(
        spawn_json["calls"][1]["payload"]["payload_ref"],
        "blob://openview/tasks/task-1/input"
    );
    let spawn_sql = spawn_json["calls"][0]["payload"]["statements"][1]["sql"]
        .as_str()
        .expect("spawn SQL");
    assert!(spawn_sql.contains("durable_tasks"));
    assert!(spawn_sql.contains("payload_ref"));
    assert!(spawn_sql.contains("ON CONFLICT"));

    assert_eq!(claim_json["required_workers"], json!(["iii-database"]));
    assert_eq!(
        claim_json["calls"][0]["function_id"],
        "iii-database::transaction"
    );
    let claim_sql = claim_json["calls"][0]["payload"]["statements"][1]["sql"]
        .as_str()
        .expect("claim SQL");
    assert!(claim_sql.contains("lease_owner = ?"));
    assert!(claim_sql.contains("visible_at <= ?"));
    assert!(claim_sql.contains("RETURNING task_id"));
    assert_eq!(
        claim_json["calls"][0]["payload"]["statements"][1]["params"][0],
        "lease-1"
    );
    assert_eq!(
        claim_json["calls"][0]["payload"]["statements"][1]["params"][1],
        "worker-a"
    );

    assert_eq!(
        heartbeat_json["calls"][0]["function_id"],
        "iii-database::execute"
    );
    let heartbeat_sql = heartbeat_json["calls"][0]["payload"]["sql"]
        .as_str()
        .expect("heartbeat SQL");
    assert!(heartbeat_sql.contains("last_heartbeat_at"));
    assert!(heartbeat_sql.contains("lease_id = ?"));
    assert!(heartbeat_sql.contains("lease_owner = ?"));
    assert_eq!(
        heartbeat_json["calls"][0]["payload"]["returning"],
        json!([
            "task_id",
            "lease_id",
            "lease_owner",
            "lease_expires_at",
            "last_heartbeat_at"
        ])
    );

    assert_serialized_without_secret_material(&spawn_json);
    assert_serialized_without_secret_material(&claim_json);
    assert_serialized_without_secret_material(&heartbeat_json);
}

#[test]
fn queue_complete_fail_retry_dead_letter_and_recovery_plans_publish_without_raw_payloads() {
    let complete = openview_queue_task_complete_plan(
        "openview",
        "task-1",
        "lease-1",
        "worker-a",
        "blob://openview/tasks/task-1/output",
        "sha256:output",
    );
    let retry = openview_queue_task_fail_retry_plan(OpenViewQueueRetryPlanRequest {
        db: "openview".to_string(),
        queue: "agent-runs".to_string(),
        task_id: "task-1".to_string(),
        lease_id: "lease-1".to_string(),
        worker_id: "worker-a".to_string(),
        failure_ref: "blob://openview/tasks/task-1/failure".to_string(),
        failure_kind: "transient_exit".to_string(),
        next_attempt: 2,
        retry_at: "2026-05-15T10:05:00Z".to_string(),
        max_attempts: 4,
    });
    let dead_letter = openview_queue_task_fail_dead_letter_plan(
        "openview",
        "task-1",
        "lease-1",
        "worker-a",
        "blob://openview/tasks/task-1/failure",
        "policy_denied",
    );
    let recovery = openview_queue_recovery_scan_plan("openview", "agent-runs", "recovery-1", 25);

    let complete_json = serde_json::to_value(&complete).expect("complete plan serializes");
    let retry_json = serde_json::to_value(&retry).expect("retry plan serializes");
    let dead_letter_json = serde_json::to_value(&dead_letter).expect("dead-letter plan serializes");
    let recovery_json = serde_json::to_value(&recovery).expect("recovery plan serializes");

    let complete_sql = complete_json["calls"][0]["payload"]["statements"][0]["sql"]
        .as_str()
        .expect("complete SQL");
    assert!(complete_sql.contains("state = 'completed'"));
    assert!(complete_sql.contains("lease_id = ?"));
    assert!(complete_sql.contains("lease_owner = ?"));
    assert_eq!(
        complete_json["calls"][1]["function_id"],
        "iii::durable::publish"
    );
    assert_eq!(
        complete_json["calls"][1]["payload"]["topic"],
        "openview.queue.task.completed"
    );
    assert_eq!(
        complete_json["calls"][1]["payload"]["data"]["output_ref"],
        "blob://openview/tasks/task-1/output"
    );

    let retry_sql = retry_json["calls"][0]["payload"]["statements"][0]["sql"]
        .as_str()
        .expect("retry SQL");
    assert!(retry_sql.contains("state = 'retrying'"));
    assert!(retry_sql.contains("attempt + 1 < ?"));
    assert_eq!(
        retry_json["calls"][1]["function_id"],
        "openview.queue::dispatch"
    );
    assert_eq!(
        retry_json["calls"][1]["action"],
        json!({"kind": "enqueue", "queue": "agent-runs"})
    );
    assert_eq!(retry_json["calls"][1]["payload"]["reason"], "retry");
    assert_eq!(
        retry_json["calls"][1]["payload"]["not_before"],
        "2026-05-15T10:05:00Z"
    );

    let dead_letter_sql = dead_letter_json["calls"][0]["payload"]["statements"][0]["sql"]
        .as_str()
        .expect("dead-letter SQL");
    assert!(dead_letter_sql.contains("state = 'dead_lettered'"));
    assert!(dead_letter_sql.contains("lease_owner = ?"));
    assert!(
        dead_letter_json["calls"][0]["payload"]["statements"][1]["sql"]
            .as_str()
            .expect("dead-letter insert SQL")
            .contains("durable_dead_letters")
    );
    assert_eq!(
        dead_letter_json["calls"][1]["payload"]["topic"],
        "openview.queue.task.dead_lettered"
    );

    assert_eq!(
        recovery_json["calls"][0]["function_id"],
        "iii-database::query"
    );
    assert_eq!(
        recovery_json["calls"][1]["function_id"],
        "iii-database::execute"
    );
    assert_eq!(
        recovery_json["calls"][2]["function_id"],
        "openview.queue::dispatch"
    );
    let recovery_query = recovery_json["calls"][0]["payload"]["sql"]
        .as_str()
        .expect("recovery query SQL");
    assert!(recovery_query.contains("lease_expires_at <= ?"));
    assert!(recovery_query.contains("visible_at <= ?"));
    let recovery_execute = recovery_json["calls"][1]["payload"]["sql"]
        .as_str()
        .expect("recovery execute SQL");
    assert!(recovery_execute.contains("recovered_by = ?"));
    assert!(recovery_execute.contains("LIMIT ?"));
    assert_eq!(
        recovery_json["calls"][2]["action"],
        json!({"kind": "enqueue", "queue": "agent-runs"})
    );
    assert_eq!(
        recovery_json["calls"][2]["payload"]["reason"],
        "recovery_scan"
    );

    assert_serialized_without_secret_material(&complete_json);
    assert_serialized_without_secret_material(&retry_json);
    assert_serialized_without_secret_material(&dead_letter_json);
    assert_serialized_without_secret_material(&recovery_json);
}

#[test]
fn queue_schema_bootstrap_plan_declares_owned_tables_indexes_and_idempotent_sql() {
    let plan = openview_queue_schema_bootstrap_plan("openview");
    let serialized = serde_json::to_value(&plan).expect("schema bootstrap plan serializes");

    assert_eq!(serialized["required_workers"], json!(["iii-database"]));
    assert_eq!(serialized["resources"], json!(["database"]));
    assert_eq!(serialized["risks"], json!(["write", "state"]));
    assert!(serialized["purpose"]
        .as_str()
        .expect("purpose")
        .contains(OPENVIEW_QUEUE_SCHEMA_VERSION));
    assert_eq!(
        serialized["calls"][0]["function_id"],
        "iii-database::transaction"
    );
    assert_eq!(serialized["calls"][0]["payload"]["db"], "openview");

    let statements = serialized["calls"][0]["payload"]["statements"]
        .as_array()
        .expect("bootstrap statements");
    let sql = statements
        .iter()
        .map(|statement| statement["sql"].as_str().expect("bootstrap statement sql"))
        .collect::<Vec<_>>();

    for table in OPENVIEW_QUEUE_TABLE_NAMES {
        assert!(
            sql.iter().any(|statement| statement
                .contains(&format!("CREATE TABLE IF NOT EXISTS {table}"))),
            "missing idempotent create table for {table}: {sql:?}"
        );
    }

    for index in OPENVIEW_QUEUE_INDEX_NAMES {
        assert!(
            sql.iter().any(|statement| statement
                .contains(&format!("CREATE INDEX IF NOT EXISTS {index}"))),
            "missing idempotent create index for {index}: {sql:?}"
        );
    }

    for statement in &sql {
        if statement.starts_with("CREATE TABLE") || statement.starts_with("CREATE INDEX") {
            assert!(
                statement.contains("IF NOT EXISTS"),
                "DDL must be idempotent: {statement}"
            );
        }
    }

    let durable_tasks = sql
        .iter()
        .find(|statement| statement.contains("CREATE TABLE IF NOT EXISTS durable_tasks"))
        .expect("durable_tasks table definition");
    for column in [
        "task_id TEXT PRIMARY KEY",
        "queue TEXT NOT NULL",
        "state TEXT NOT NULL",
        "visible_at TEXT NOT NULL",
        "lease_id TEXT",
        "lease_owner TEXT",
        "lease_expires_at TEXT",
        "recovered_by TEXT",
    ] {
        assert!(durable_tasks.contains(column), "missing {column}");
    }

    let claim_index = sql
        .iter()
        .find(|statement| statement.contains("idx_durable_tasks_claim_ready"))
        .expect("claim-ready index");
    assert!(claim_index.contains("queue, state, visible_at, created_at"));
    assert!(claim_index.contains("WHERE state IN ('queued', 'retrying')"));

    let lease_recovery_index = sql
        .iter()
        .find(|statement| statement.contains("idx_durable_tasks_recover_leases"))
        .expect("lease recovery index");
    assert!(lease_recovery_index.contains("queue, state, lease_expires_at, created_at"));
    assert!(lease_recovery_index.contains("WHERE state = 'leased'"));

    let retry_recovery_index = sql
        .iter()
        .find(|statement| statement.contains("idx_durable_tasks_recover_retries"))
        .expect("retry recovery index");
    assert!(retry_recovery_index.contains("queue, state, visible_at, created_at"));
    assert!(retry_recovery_index.contains("WHERE state = 'retrying'"));

    let idempotency_keys = sql
        .iter()
        .find(|statement| statement.contains("CREATE TABLE IF NOT EXISTS durable_idempotency_keys"))
        .expect("idempotency table definition");
    assert!(idempotency_keys.contains("idempotency_key TEXT PRIMARY KEY"));
    assert!(idempotency_keys.contains("task_id TEXT NOT NULL"));
    assert!(!idempotency_keys.contains("FOREIGN KEY"));

    assert!(statements
        .iter()
        .all(|statement| statement["params"] == json!([])));
    assert_serialized_without_secret_material(&serialized);
}

#[test]
fn serializes_plans_with_resources_risks_and_no_live_network_requirement() {
    let plan = IiiTriggerPlan::new(
        "Persist an OpenView task event and notify hooks",
        vec![
            iii_database_execute_invocation(
                "primary",
                "INSERT INTO events(body) VALUES (?)",
                vec![json!("queued")],
                vec!["id"],
            ),
            hook_fanout_publish_collect_invocation(
                "openview::task_event",
                json!({"id": "evt_1"}),
                "collect_all",
                1_000,
            ),
        ],
    )
    .worker("iii-database")
    .worker("hook-fanout")
    .resource(ResourceKind::Database)
    .resource(ResourceKind::HookTopic)
    .risk(CapabilityRisk::Write)
    .risk(CapabilityRisk::Stream);

    let serialized = serde_json::to_value(&plan).expect("plan serializes");

    assert_eq!(
        serialized["purpose"],
        "Persist an OpenView task event and notify hooks"
    );
    assert_eq!(serialized["calls"].as_array().unwrap().len(), 2);
    assert_eq!(
        serialized["required_workers"],
        json!(["iii-database", "hook-fanout"])
    );
    assert_eq!(serialized["resources"], json!(["database", "hook_topic"]));
    assert_eq!(serialized["risks"], json!(["write", "stream"]));
}

#[test]
fn manifest_exposes_openview_iii_backend_adapter_resources_risks_and_sandbox_handoff() {
    let manifest = openview_iii_backend_adapter_manifest();
    let function_ids = manifest
        .functions
        .iter()
        .map(|function| function.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(manifest.name, "openview.iii.backend");
    assert!(manifest.targets.contains(&BackendTarget::ThreeICompatible));
    assert!(manifest.resources.contains(&ResourceKind::Database));
    assert!(manifest.resources.contains(&ResourceKind::EventStream));
    assert!(manifest.resources.contains(&ResourceKind::HookTopic));
    assert!(manifest.resources.contains(&ResourceKind::ApprovalQueue));
    assert!(manifest.resources.contains(&ResourceKind::SessionState));
    assert!(manifest.resources.contains(&ResourceKind::Process));
    assert!(manifest.resources.contains(&ResourceKind::Sandbox));
    assert!(manifest.dependencies.contains("iii-queue"));
    assert!(manifest.dependencies.contains("iii-database"));
    assert!(manifest.dependencies.contains("hook-fanout"));
    assert!(manifest.dependencies.contains("approval-gate"));
    assert!(manifest.dependencies.contains("turn-orchestrator"));
    assert!(manifest.dependencies.contains("session-tree"));
    assert!(manifest.dependencies.contains("shell"));

    for required in [
        OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION,
        "openview.queue::spawn_task",
        "openview.queue::dispatch",
        "openview.queue::claim_lease",
        "openview.queue::heartbeat_lease",
        "openview.queue::complete_task",
        "openview.queue::fail_retry",
        "openview.queue::fail_dead_letter",
        "openview.queue::recovery_scan",
        "iii-database::transaction",
        "iii-database::query",
        "iii-database::execute",
        "iii::durable::publish",
        "stream::set",
        "stream::list",
        "hook-fanout::publish_collect",
        "approval::resolve",
        "approval::list_pending",
        "run::start",
        "run::start_and_wait",
        "session-tree::create",
        "session-tree::append",
        "session-tree::messages",
        "shell::exec",
        "sandbox::exec",
    ] {
        assert!(function_ids.contains(&required), "missing {required}");
    }

    let execute = manifest
        .functions
        .iter()
        .find(|f| f.id == "iii-database::execute")
        .unwrap();
    assert_eq!(execute.risk, CapabilityRisk::Write);
    assert!(execute.approval_required);

    let query = manifest
        .functions
        .iter()
        .find(|f| f.id == "iii-database::query")
        .unwrap();
    assert_eq!(query.risk, CapabilityRisk::Read);
    assert!(!query.approval_required);

    let bootstrap_schema = manifest
        .functions
        .iter()
        .find(|f| f.id == OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION)
        .unwrap();
    assert_eq!(bootstrap_schema.risk, CapabilityRisk::Write);
    assert!(bootstrap_schema.approval_required);
    assert_eq!(
        bootstrap_schema.request_schema["properties"]["schema_version"]["enum"],
        json!([OPENVIEW_QUEUE_SCHEMA_VERSION])
    );
    assert_eq!(
        bootstrap_schema.request_schema["properties"]["migration_id"]["enum"],
        json!([OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID])
    );
    assert_eq!(
        bootstrap_schema.response_schema["required"],
        json!(["schema_version", "migration_id", "tables", "indexes"])
    );

    let spawn = manifest
        .functions
        .iter()
        .find(|f| f.id == "openview.queue::spawn_task")
        .unwrap();
    assert_eq!(spawn.risk, CapabilityRisk::Write);
    assert!(spawn.request_schema["properties"]
        .get("payload_ref")
        .is_some());
    assert!(spawn.request_schema["properties"].get("payload").is_none());

    let dispatch = manifest
        .functions
        .iter()
        .find(|f| f.id == "openview.queue::dispatch")
        .unwrap();
    assert_eq!(dispatch.visibility, FunctionVisibility::Internal);
    assert_eq!(dispatch.risk, CapabilityRisk::State);

    let recovery = manifest
        .functions
        .iter()
        .find(|f| f.id == "openview.queue::recovery_scan")
        .unwrap();
    assert_eq!(recovery.risk, CapabilityRisk::Write);
    assert_eq!(
        recovery.request_schema["required"],
        json!(["db", "queue", "recovery_owner", "limit"])
    );

    let shell = manifest
        .functions
        .iter()
        .find(|f| f.id == "shell::exec")
        .unwrap();
    assert_eq!(shell.risk, CapabilityRisk::Exec);
    assert!(shell.approval_required);
    assert_eq!(
        shell.request_schema["properties"]["target"]["properties"]["kind"]["enum"],
        json!(["sandbox"])
    );

    assert!(manifest
        .security
        .approval_required_functions
        .contains("shell::exec"));
    assert!(manifest
        .security
        .approval_required_functions
        .contains(OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION));
    assert!(manifest
        .security
        .approval_required_functions
        .contains("sandbox::exec"));
    assert_eq!(manifest.default_config["network_required"], false);
    assert_eq!(
        manifest.default_config["queue_dispatch_function"],
        "openview.queue::dispatch"
    );
    assert_eq!(
        manifest.default_config["queue_schema_version"],
        OPENVIEW_QUEUE_SCHEMA_VERSION
    );
    assert_eq!(
        manifest.default_config["queue_schema_migration_function"],
        OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION
    );
    assert_eq!(
        manifest.default_config["queue_schema_migration_id"],
        OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID
    );
    assert_eq!(
        manifest.default_config["queue_tables"],
        json!(OPENVIEW_QUEUE_TABLE_NAMES)
    );
    assert_eq!(
        manifest.default_config["queue_indexes"],
        json!(OPENVIEW_QUEUE_INDEX_NAMES)
    );
    assert_eq!(
        manifest.default_config["queue_schema_migrations"][0]["version"],
        OPENVIEW_QUEUE_SCHEMA_VERSION
    );
    assert_eq!(
        manifest.default_config["queue_schema_migrations"][0]["function"],
        OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION
    );
}

#[test]
fn raw_invocation_builder_keeps_contract_only_shape() {
    let invocation = IiiTriggerInvocation::new(
        "stream::set",
        json!({
            "stream_name": "openview::events",
            "group_id": "run-1",
            "item_id": "evt-1",
            "data": {"ok": true}
        }),
    )
    .timeout_ms(500);

    let serialized = serde_json::to_value(invocation).expect("invocation serializes");

    assert_eq!(serialized["function_id"], "stream::set");
    assert_eq!(serialized["action"], serde_json::Value::Null);
    assert_eq!(serialized["timeout_ms"], 500);
    assert_eq!(serialized["payload"]["data"]["ok"], true);
}
