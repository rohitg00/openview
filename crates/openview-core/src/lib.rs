use chrono::{DateTime, Utc};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

pub type WorkerId = String;
pub type FunctionId = String;
pub type AgentId = String;

#[derive(Debug, Error)]
pub enum OpenViewError {
    #[error("worker already registered: {0}")]
    DuplicateWorker(String),
    #[error("worker not found: {0}")]
    WorkerNotFound(String),
    #[error("agent not found: {0}")]
    AgentNotFound(String),
    #[error("approval not found: {0}")]
    ApprovalNotFound(Uuid),
    #[error("run not found: {0}")]
    RunNotFound(Uuid),
    #[error("graph has no agents")]
    EmptyGraph,
    #[error("workspace has no tabs")]
    EmptyWorkspace,
    #[error("workspace active pane not found: {0}")]
    UnknownPane(String),
    #[error("handoff references unknown agent: {0}")]
    UnknownHandoffAgent(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendTarget {
    LocalBinary,
    RemoteBinary,
    DurableFunction,
    Browser,
    Desktop,
    Mobile,
    McpServer,
    ThreeICompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRisk {
    Read,
    Write,
    Exec,
    Network,
    Credential,
    Approval,
    Budget,
    Model,
    State,
    Stream,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Sandbox,
    Filesystem,
    Network,
    Process,
    CredentialVault,
    ModelProvider,
    ModelRouter,
    ApprovalQueue,
    SessionState,
    EventStream,
    ObjectStorage,
    Database,
    HookTopic,
    BudgetLedger,
    PolicySet,
    Terminal,
    GitWorktree,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionPolicy {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub allow: bool,
    pub hosts: IndexSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemPolicy {
    pub read_roots: IndexSet<String>,
    pub write_roots: IndexSet<String>,
    pub deny_roots: IndexSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessPolicy {
    pub allow_exec: bool,
    pub allowed_commands: IndexSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxProfile {
    pub name: String,
    pub filesystem: FilesystemPolicy,
    pub network: NetworkPolicy,
    pub process: ProcessPolicy,
    pub secret_names: IndexSet<String>,
}

impl SandboxProfile {
    pub fn locked_down() -> Self {
        Self {
            name: "locked-down".to_string(),
            filesystem: FilesystemPolicy {
                read_roots: IndexSet::new(),
                write_roots: IndexSet::new(),
                deny_roots: IndexSet::from(["/".to_string()]),
            },
            network: NetworkPolicy {
                allow: false,
                hosts: IndexSet::new(),
            },
            process: ProcessPolicy {
                allow_exec: false,
                allowed_commands: IndexSet::new(),
            },
            secret_names: IndexSet::new(),
        }
    }

    pub fn allow_workspace_read(mut self, root: impl Into<String>) -> Self {
        self.filesystem.read_roots.insert(root.into());
        self
    }

    pub fn allow_workspace_write(mut self, root: impl Into<String>) -> Self {
        self.filesystem.write_roots.insert(root.into());
        self
    }

    pub fn allow_command(mut self, command: impl Into<String>) -> Self {
        self.process.allow_exec = true;
        self.process.allowed_commands.insert(command.into());
        self
    }

    pub fn allow_network_host(mut self, host: impl Into<String>) -> Self {
        self.network.allow = true;
        self.network.hosts.insert(host.into());
        self
    }

    pub fn deny_network(mut self) -> Self {
        self.network.allow = false;
        self.network.hosts.clear();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionSpec {
    pub id: FunctionId,
    pub description: String,
    pub risk: CapabilityRisk,
    pub request_schema: Value,
    pub response_schema: Value,
    pub approval_required: bool,
    pub visibility: FunctionVisibility,
}

impl FunctionSpec {
    pub fn new(id: impl Into<String>, risk: CapabilityRisk) -> Self {
        Self {
            id: id.into(),
            description: String::new(),
            risk,
            request_schema: json!({"type":"object"}),
            response_schema: json!({"type":"object"}),
            approval_required: false,
            visibility: FunctionVisibility::Agent,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn approval_required(mut self, required: bool) -> Self {
        self.approval_required = required;
        self
    }

    pub fn request_schema(mut self, schema: Value) -> Self {
        self.request_schema = schema;
        self
    }

    pub fn response_schema(mut self, schema: Value) -> Self {
        self.response_schema = schema;
        self
    }

    pub fn visibility(mut self, visibility: FunctionVisibility) -> Self {
        self.visibility = visibility;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionVisibility {
    Internal,
    Ui,
    Agent,
    Mcp,
    Http,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerSpec {
    pub trigger_type: String,
    pub function_id: FunctionId,
    pub config: Value,
    pub metadata: IndexMap<String, String>,
}

impl TriggerSpec {
    pub fn http(function_id: impl Into<String>, api_path: impl Into<String>) -> Self {
        Self {
            trigger_type: "http".to_string(),
            function_id: function_id.into(),
            config: json!({"api_path": api_path.into(), "http_method": "POST"}),
            metadata: IndexMap::new(),
        }
    }

    pub fn durable_subscriber(function_id: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            trigger_type: "durable:subscriber".to_string(),
            function_id: function_id.into(),
            config: json!({"topic": topic.into(), "action": "enqueue"}),
            metadata: IndexMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub default_policy: PermissionPolicy,
    pub approval_required_functions: IndexSet<FunctionId>,
    pub denied_functions: IndexSet<FunctionId>,
    pub notes: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            default_policy: PermissionPolicy::Deny,
            approval_required_functions: IndexSet::new(),
            denied_functions: IndexSet::new(),
            notes: vec![
                "default deny until a function, trigger, or worker declares narrower scope".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerManifest {
    pub schema_version: String,
    pub name: WorkerId,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub language: String,
    pub deploy_kind: String,
    pub binary_name: Option<String>,
    pub targets: IndexSet<BackendTarget>,
    pub functions: Vec<FunctionSpec>,
    pub triggers: Vec<TriggerSpec>,
    pub resources: IndexSet<ResourceKind>,
    pub dependencies: IndexSet<WorkerId>,
    pub default_config: Value,
    pub security: SecurityPolicy,
    pub sandbox: Option<SandboxProfile>,
}

impl WorkerManifest {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            schema_version: "openview.worker.v1".to_string(),
            display_name: name.clone(),
            name,
            version: version.into(),
            description: String::new(),
            language: "rust".to_string(),
            deploy_kind: "binary".to_string(),
            binary_name: None,
            targets: IndexSet::new(),
            functions: Vec::new(),
            triggers: Vec::new(),
            resources: IndexSet::new(),
            dependencies: IndexSet::new(),
            default_config: json!({}),
            security: SecurityPolicy::default(),
            sandbox: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn target(mut self, target: BackendTarget) -> Self {
        self.targets.insert(target);
        self
    }

    pub fn function(mut self, function: FunctionSpec) -> Self {
        if function.approval_required {
            self.security
                .approval_required_functions
                .insert(function.id.clone());
        }
        self.functions.push(function);
        self
    }

    pub fn trigger(mut self, trigger: TriggerSpec) -> Self {
        self.triggers.push(trigger);
        self
    }

    pub fn resource(mut self, resource: ResourceKind) -> Self {
        self.resources.insert(resource);
        self
    }

    pub fn dependency(mut self, worker_id: impl Into<String>) -> Self {
        self.dependencies.insert(worker_id.into());
        self
    }

    pub fn sandbox(mut self, sandbox: SandboxProfile) -> Self {
        self.resources.insert(ResourceKind::Sandbox);
        self.sandbox = Some(sandbox);
        self
    }

    pub fn to_backend_messages(&self) -> Vec<BackendMessage> {
        let mut messages = vec![BackendMessage::RegisterWorker {
            worker_id: self.name.clone(),
            version: self.version.clone(),
            metadata: json!({
                "display_name": self.display_name,
                "resources": self.resources,
                "targets": self.targets,
            }),
        }];
        for function in &self.functions {
            messages.push(BackendMessage::RegisterFunction {
                function_id: function.id.clone(),
                description: function.description.clone(),
                request_schema: function.request_schema.clone(),
                response_schema: function.response_schema.clone(),
                metadata: json!({
                    "worker_id": self.name,
                    "risk": function.risk,
                    "approval_required": function.approval_required,
                    "visibility": function.visibility,
                }),
            });
        }
        for trigger in &self.triggers {
            messages.push(BackendMessage::RegisterTrigger {
                trigger_type: trigger.trigger_type.clone(),
                function_id: trigger.function_id.clone(),
                config: trigger.config.clone(),
                metadata: json!(trigger.metadata),
            });
        }
        messages
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackendMessage {
    RegisterWorker {
        worker_id: WorkerId,
        version: String,
        metadata: Value,
    },
    RegisterFunction {
        function_id: FunctionId,
        description: String,
        request_schema: Value,
        response_schema: Value,
        metadata: Value,
    },
    RegisterTrigger {
        trigger_type: String,
        function_id: FunctionId,
        config: Value,
        metadata: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneKind {
    AgentRun,
    Terminal,
    Editor,
    Browser,
    Notes,
    ApprovalQueue,
    WorkerCatalog,
    TraceTimeline,
    Worktree,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePane {
    pub id: String,
    pub kind: PaneKind,
    pub title: String,
    pub resource: Option<ResourceKind>,
    pub active_run: Option<Uuid>,
}

impl WorkspacePane {
    pub fn new(id: impl Into<String>, kind: PaneKind, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            title: title.into(),
            resource: None,
            active_run: None,
        }
    }

    pub fn resource(mut self, resource: ResourceKind) -> Self {
        self.resource = Some(resource);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTab {
    pub id: String,
    pub title: String,
    pub panes: Vec<WorkspacePane>,
    pub active_pane_id: Option<String>,
}

impl WorkspaceTab {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            panes: Vec::new(),
            active_pane_id: None,
        }
    }

    pub fn pane(mut self, pane: WorkspacePane) -> Self {
        if self.active_pane_id.is_none() {
            self.active_pane_id = Some(pane.id.clone());
        }
        self.panes.push(pane);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeBinding {
    pub id: String,
    pub repo_path: String,
    pub branch: String,
    pub base_ref: Option<String>,
    pub head_sha: Option<String>,
    pub linked_run: Option<Uuid>,
}

impl WorktreeBinding {
    pub fn new(
        id: impl Into<String>,
        repo_path: impl Into<String>,
        branch: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            repo_path: repo_path.into(),
            branch: branch.into(),
            base_ref: None,
            head_sha: None,
            linked_run: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSession {
    pub id: Uuid,
    pub name: String,
    pub tabs: Vec<WorkspaceTab>,
    pub worktrees: Vec<WorktreeBinding>,
    pub restored_runs: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkspaceSession {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            tabs: Vec::new(),
            worktrees: Vec::new(),
            restored_runs: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn tab(mut self, tab: WorkspaceTab) -> Self {
        self.tabs.push(tab);
        self.updated_at = Utc::now();
        self
    }

    pub fn worktree(mut self, worktree: WorktreeBinding) -> Self {
        self.worktrees.push(worktree);
        self.updated_at = Utc::now();
        self
    }

    pub fn validate(&self) -> Result<(), OpenViewError> {
        if self.tabs.is_empty() {
            return Err(OpenViewError::EmptyWorkspace);
        }
        for tab in &self.tabs {
            if let Some(active) = &tab.active_pane_id {
                if !tab.panes.iter().any(|pane| &pane.id == active) {
                    return Err(OpenViewError::UnknownPane(active.clone()));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerHealthState {
    Unknown,
    Starting,
    Ready,
    Degraded,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerRuntimeSpec {
    pub worker_id: WorkerId,
    pub binary: String,
    pub args: Vec<String>,
    pub env_keys: Vec<String>,
    pub working_directory: Option<String>,
    pub health_state: WorkerHealthState,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
}

impl WorkerRuntimeSpec {
    pub fn new(worker_id: impl Into<String>, binary: impl Into<String>) -> Self {
        Self {
            worker_id: worker_id.into(),
            binary: binary.into(),
            args: Vec::new(),
            env_keys: Vec::new(),
            working_directory: None,
            health_state: WorkerHealthState::Unknown,
            last_heartbeat_at: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn env_key(mut self, key: impl Into<String>) -> Self {
        self.env_keys.push(key.into());
        self
    }

    pub fn ready(mut self) -> Self {
        self.health_state = WorkerHealthState::Ready;
        self.last_heartbeat_at = Some(Utc::now());
        self
    }
}

#[derive(Debug, Default, Clone)]
pub struct WorkerRegistry {
    workers: IndexMap<WorkerId, WorkerManifest>,
}

impl WorkerRegistry {
    pub fn register(&mut self, manifest: WorkerManifest) -> Result<(), OpenViewError> {
        if self.workers.contains_key(&manifest.name) {
            return Err(OpenViewError::DuplicateWorker(manifest.name));
        }
        self.workers.insert(manifest.name.clone(), manifest);
        Ok(())
    }

    pub fn worker(&self, id: &str) -> Option<&WorkerManifest> {
        self.workers.get(id)
    }

    pub fn workers(&self) -> impl Iterator<Item = &WorkerManifest> {
        self.workers.values()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBlueprint {
    pub id: AgentId,
    pub instructions: String,
    pub workers: IndexSet<WorkerId>,
}

impl AgentBlueprint {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            instructions: String::new(),
            workers: IndexSet::new(),
        }
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = instructions.into();
        self
    }

    pub fn uses_worker(mut self, worker_id: impl Into<String>) -> Self {
        self.workers.insert(worker_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    LocalInline,
    DurableQueued,
    RemoteStreamed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledGraph {
    pub name: String,
    pub execution_mode: ExecutionMode,
    pub agents: Vec<AgentBlueprint>,
    pub edges: Vec<(AgentId, AgentId)>,
    pub required_workers: IndexSet<WorkerId>,
    pub shared_resources: IndexSet<ResourceKind>,
}

#[derive(Debug, Default, Clone)]
pub struct ConnectionGraph {
    name: String,
    agents: IndexMap<AgentId, AgentBlueprint>,
    edges: Vec<(AgentId, AgentId)>,
    approval_gates: IndexMap<AgentId, WorkerId>,
    shared_resources: IndexSet<ResourceKind>,
}

impl ConnectionGraph {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub fn agent(&mut self, agent: AgentBlueprint) -> &mut Self {
        self.agents.insert(agent.id.clone(), agent);
        self
    }

    pub fn handoff(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self {
        self.edges.push((from.into(), to.into()));
        self
    }

    pub fn approval_gate(
        &mut self,
        agent: impl Into<String>,
        worker: impl Into<String>,
    ) -> &mut Self {
        self.approval_gates.insert(agent.into(), worker.into());
        self
    }

    pub fn shared_resource(&mut self, resource: ResourceKind) -> &mut Self {
        self.shared_resources.insert(resource);
        self
    }

    pub fn compile(&self) -> Result<CompiledGraph, OpenViewError> {
        if self.agents.is_empty() {
            return Err(OpenViewError::EmptyGraph);
        }
        for (from, to) in &self.edges {
            if !self.agents.contains_key(from) {
                return Err(OpenViewError::UnknownHandoffAgent(from.clone()));
            }
            if !self.agents.contains_key(to) {
                return Err(OpenViewError::UnknownHandoffAgent(to.clone()));
            }
        }
        let mut required_workers = IndexSet::new();
        for agent in self.agents.values() {
            required_workers.extend(agent.workers.iter().cloned());
        }
        required_workers.extend(self.approval_gates.values().cloned());
        Ok(CompiledGraph {
            name: self.name.clone(),
            execution_mode: ExecutionMode::DurableQueued,
            agents: self.agents.values().cloned().collect(),
            edges: self.edges.clone(),
            required_workers,
            shared_resources: self.shared_resources.clone(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPhase {
    Queued,
    Provisioning,
    Running,
    WaitingForApproval,
    FunctionCalling,
    Streaming,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub id: Uuid,
    pub worker_id: WorkerId,
    pub function_id: FunctionId,
    pub reason: String,
    pub resolved_reason: Option<String>,
    pub approved: bool,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunEvent {
    pub sequence: u64,
    pub kind: String,
    pub at: DateTime<Utc>,
    pub payload: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCursor {
    pub run_id: Uuid,
    pub after_sequence: u64,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventPage {
    pub run_id: Uuid,
    pub events: Vec<RunEvent>,
    pub next_after_sequence: u64,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepRecord {
    pub id: Uuid,
    pub worker_id: WorkerId,
    pub phase: RunPhase,
    pub function_ids: Vec<FunctionId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: Uuid,
    pub goal: String,
    pub phase: RunPhase,
    pub workers: Vec<WorkerId>,
    pub steps: Vec<StepRecord>,
    pub approvals: Vec<ApprovalRecord>,
    pub events: Vec<RunEvent>,
}

#[derive(Debug, Default)]
pub struct OpenViewRuntime {
    registry: WorkerRegistry,
    runs: IndexMap<Uuid, RunRecord>,
}

impl OpenViewRuntime {
    pub fn register_worker(&mut self, manifest: WorkerManifest) -> Result<(), OpenViewError> {
        self.registry.register(manifest)
    }

    pub fn registry(&self) -> &WorkerRegistry {
        &self.registry
    }

    pub fn run(&self, run_id: Uuid) -> Option<&RunRecord> {
        self.runs.get(&run_id)
    }

    pub fn events_since(
        &self,
        run_id: Uuid,
        after_sequence: u64,
        limit: usize,
    ) -> Result<EventPage, OpenViewError> {
        let run = self
            .runs
            .get(&run_id)
            .ok_or(OpenViewError::RunNotFound(run_id))?;
        let events = run
            .events
            .iter()
            .filter(|event| event.sequence > after_sequence)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_after_sequence = events
            .last()
            .map(|event| event.sequence)
            .unwrap_or(after_sequence);
        let has_more = run
            .events
            .iter()
            .any(|event| event.sequence > next_after_sequence);

        Ok(EventPage {
            run_id,
            events,
            next_after_sequence,
            has_more,
        })
    }

    pub fn start_run<I, S>(
        &mut self,
        goal: impl Into<String>,
        workers: I,
    ) -> Result<RunRecord, OpenViewError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut run = RunRecord {
            id: Uuid::new_v4(),
            goal: goal.into(),
            phase: RunPhase::Running,
            workers: Vec::new(),
            steps: Vec::new(),
            approvals: Vec::new(),
            events: Vec::new(),
        };
        let goal_payload = run.goal.clone();
        self.emit(&mut run, "run.started", json!({"goal": goal_payload}));
        for worker_id in workers.into_iter().map(Into::into) {
            let worker = self
                .registry
                .worker(&worker_id)
                .ok_or_else(|| OpenViewError::WorkerNotFound(worker_id.clone()))?;
            run.workers.push(worker_id.clone());
            let function_ids = worker
                .functions
                .iter()
                .map(|f| f.id.clone())
                .collect::<Vec<_>>();
            let mut step = StepRecord {
                id: Uuid::new_v4(),
                worker_id: worker_id.clone(),
                phase: RunPhase::Running,
                function_ids,
            };
            self.emit(&mut run, "worker.started", json!({"worker_id": worker_id}));
            if let Some(function) = worker.functions.iter().find(|f| f.approval_required) {
                let approval = ApprovalRecord {
                    id: Uuid::new_v4(),
                    worker_id: worker_id.clone(),
                    function_id: function.id.clone(),
                    reason: format!(
                        "{} requires approval before {:?}",
                        function.id, function.risk
                    ),
                    resolved_reason: None,
                    approved: false,
                    created_at: Utc::now(),
                    resolved_at: None,
                };
                self.emit(&mut run, "approval.requested", json!({"approval_id": approval.id, "worker_id": worker_id, "function_id": function.id}));
                step.phase = RunPhase::WaitingForApproval;
                run.steps.push(step);
                run.approvals.push(approval);
                run.phase = RunPhase::WaitingForApproval;
                self.runs.insert(run.id, run.clone());
                return Ok(run);
            }
            step.phase = RunPhase::Completed;
            run.steps.push(step);
            self.emit(
                &mut run,
                "worker.completed",
                json!({"worker_id": worker_id}),
            );
        }
        run.phase = RunPhase::Completed;
        let run_id_payload = run.id;
        self.emit(&mut run, "run.completed", json!({"run_id": run_id_payload}));
        self.runs.insert(run.id, run.clone());
        Ok(run)
    }

    pub fn approve(
        &mut self,
        run_id: Uuid,
        approval_id: Uuid,
        reason: impl Into<String>,
    ) -> Result<RunRecord, OpenViewError> {
        let mut run = self
            .runs
            .get(&run_id)
            .cloned()
            .ok_or(OpenViewError::ApprovalNotFound(approval_id))?;
        let reason = reason.into();
        let approval = run
            .approvals
            .iter_mut()
            .find(|a| a.id == approval_id)
            .ok_or(OpenViewError::ApprovalNotFound(approval_id))?;
        approval.approved = true;
        approval.resolved_reason = Some(reason.clone());
        approval.resolved_at = Some(Utc::now());
        self.emit(
            &mut run,
            "approval.approved",
            json!({"approval_id": approval_id, "reason": reason}),
        );
        for step in &mut run.steps {
            if step.phase == RunPhase::WaitingForApproval {
                step.phase = RunPhase::Completed;
            }
        }
        run.phase = RunPhase::Completed;
        let run_id_payload = run.id;
        self.emit(&mut run, "run.completed", json!({"run_id": run_id_payload}));
        self.runs.insert(run.id, run.clone());
        Ok(run)
    }

    fn emit(&self, run: &mut RunRecord, kind: impl Into<String>, payload: Value) {
        let sequence = run.events.len() as u64 + 1;
        run.events.push(RunEvent {
            sequence,
            kind: kind.into(),
            at: Utc::now(),
            payload,
        });
    }
}

pub fn built_in_worker_catalog() -> Vec<WorkerManifest> {
    vec![
        git_worktree_worker_manifest(),
        terminal_pty_worker_manifest(),
        WorkerManifest::new("approval.gate", "0.1.0")
            .description("Human approval queue for risky agent actions")
            .resource(ResourceKind::ApprovalQueue)
            .function(
                FunctionSpec::new("approval::request", CapabilityRisk::Approval)
                    .visibility(FunctionVisibility::Ui),
            ),
        WorkerManifest::new("shell.sandbox", "0.1.0")
            .description(
                "Sandboxed command execution with explicit filesystem, process, and network policy",
            )
            .resource(ResourceKind::Sandbox)
            .resource(ResourceKind::Process)
            .function(FunctionSpec::new("shell::run", CapabilityRisk::Exec).approval_required(true))
            .sandbox(SandboxProfile::locked_down()),
        WorkerManifest::new("turn.orchestrator", "0.1.0")
            .description("Durable turn state machine for agent sessions")
            .resource(ResourceKind::SessionState)
            .resource(ResourceKind::EventStream)
            .function(FunctionSpec::new("run::start", CapabilityRisk::State))
            .function(FunctionSpec::new(
                "run::start_and_wait",
                CapabilityRisk::State,
            )),
        WorkerManifest::new("models.router", "0.1.0")
            .description("Model/provider routing with budget-aware selection")
            .resource(ResourceKind::ModelRouter)
            .resource(ResourceKind::BudgetLedger)
            .function(FunctionSpec::new(
                "router::stream_assistant",
                CapabilityRisk::Model,
            )),
        WorkerManifest::new("policy.guard", "0.1.0")
            .description("Policy denylist and permission evaluation worker")
            .resource(ResourceKind::PolicySet)
            .function(FunctionSpec::new(
                "policy::evaluate",
                CapabilityRisk::Approval,
            )),
        WorkerManifest::new("storage.objects", "0.1.0")
            .description("Artifact and object storage worker")
            .resource(ResourceKind::ObjectStorage)
            .function(FunctionSpec::new("storage::put", CapabilityRisk::Write)),
    ]
}

pub fn terminal_pty_worker_manifest() -> WorkerManifest {
    const WORKSPACE_ROOT: &str = "${OPENVIEW_WORKSPACE_ROOT}";
    const SAFE_SHELL: &str = "${OPENVIEW_SAFE_SHELL}";
    const SAFE_AGENT_CLI: &str = "${OPENVIEW_SAFE_AGENT_CLI}";

    WorkerManifest::new("terminal.pty", "0.1.0")
        .description("Pseudo-terminal session lifecycle, input, output, resize, and teardown")
        .target(BackendTarget::LocalBinary)
        .resource(ResourceKind::Terminal)
        .resource(ResourceKind::Process)
        .resource(ResourceKind::EventStream)
        .resource(ResourceKind::Filesystem)
        .function(
            FunctionSpec::new("terminal.pty::spawn", CapabilityRisk::Exec)
                .description("Spawn an approved pseudo-terminal session")
                .request_schema(json!({
                    "type": "object",
                    "required": ["command"],
                    "properties": {
                        "command": {"type": "string", "description": "Configured shell or safe CLI command placeholder"},
                        "args": {"type": "array", "items": {"type": "string"}, "default": []},
                        "cwd": {"type": ["string", "null"], "description": "Working directory under the workspace root"},
                        "env": {"type": "object", "additionalProperties": {"type": "string"}, "default": {}},
                        "cols": {"type": "integer", "minimum": 1, "default": 80},
                        "rows": {"type": "integer", "minimum": 1, "default": 24}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["session_id", "pid", "started"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "pid": {"type": ["integer", "null"]},
                        "started": {"type": "boolean"},
                        "event_stream": {"type": ["string", "null"]}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true)
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new("terminal.pty::write", CapabilityRisk::Exec)
                .description("Write input bytes to an approved pseudo-terminal session")
                .request_schema(json!({
                    "type": "object",
                    "required": ["session_id", "data"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "data": {"type": "string", "description": "UTF-8 input to send to the terminal"},
                        "append_newline": {"type": "boolean", "default": false}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["session_id", "accepted"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "accepted": {"type": "boolean"},
                        "bytes_written": {"type": "integer", "minimum": 0}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true)
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new("terminal.pty::read", CapabilityRisk::Read)
                .description("Read buffered output from a pseudo-terminal session")
                .request_schema(json!({
                    "type": "object",
                    "required": ["session_id"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "cursor": {"type": ["integer", "null"], "minimum": 0},
                        "max_bytes": {"type": "integer", "minimum": 1, "default": 65536}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["session_id", "output", "cursor", "eof"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "output": {"type": "string"},
                        "cursor": {"type": "integer", "minimum": 0},
                        "eof": {"type": "boolean"},
                        "exit_status": {"type": ["integer", "null"]}
                    },
                    "additionalProperties": false
                }))
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new("terminal.pty::resize", CapabilityRisk::State)
                .description("Resize an existing pseudo-terminal session")
                .request_schema(json!({
                    "type": "object",
                    "required": ["session_id", "cols", "rows"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "cols": {"type": "integer", "minimum": 1},
                        "rows": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["session_id", "cols", "rows", "resized"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "cols": {"type": "integer", "minimum": 1},
                        "rows": {"type": "integer", "minimum": 1},
                        "resized": {"type": "boolean"}
                    },
                    "additionalProperties": false
                }))
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new("terminal.pty::kill", CapabilityRisk::Exec)
                .description("Terminate a pseudo-terminal session")
                .request_schema(json!({
                    "type": "object",
                    "required": ["session_id"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "signal": {"type": "string", "default": "TERM"}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["session_id", "terminated"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "terminated": {"type": "boolean"},
                        "exit_status": {"type": ["integer", "null"]}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true)
                .visibility(FunctionVisibility::Ui),
        )
        .sandbox(
            SandboxProfile::locked_down()
                .allow_workspace_read(WORKSPACE_ROOT)
                .allow_workspace_write(WORKSPACE_ROOT)
                .allow_command(SAFE_SHELL)
                .allow_command(SAFE_AGENT_CLI)
                .deny_network(),
        )
}

pub fn git_worktree_worker_manifest() -> WorkerManifest {
    const WORKSPACE_ROOT: &str = "${OPENVIEW_WORKSPACE_ROOT}";
    const WORKTREE_ROOT: &str = "${OPENVIEW_WORKTREE_ROOT}";

    WorkerManifest::new("git.worktree", "0.1.0")
        .description("Git worktree lifecycle and status operations with scoped filesystem access")
        .target(BackendTarget::LocalBinary)
        .resource(ResourceKind::GitWorktree)
        .resource(ResourceKind::Filesystem)
        .resource(ResourceKind::Process)
        .function(
            FunctionSpec::new("git.worktree::list", CapabilityRisk::Read)
                .description("List worktrees for a repository")
                .request_schema(json!({
                    "type": "object",
                    "required": ["repo_path"],
                    "properties": {
                        "repo_path": {"type": "string", "description": "Repository path under the workspace root"}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["worktrees"],
                    "properties": {
                        "worktrees": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["path"],
                                "properties": {
                                    "path": {"type": "string"},
                                    "branch": {"type": ["string", "null"]},
                                    "head": {"type": ["string", "null"]},
                                    "bare": {"type": "boolean"},
                                    "detached": {"type": "boolean"}
                                },
                                "additionalProperties": false
                            }
                        }
                    },
                    "additionalProperties": false
                }))
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new("git.worktree::create", CapabilityRisk::Write)
                .description("Create a new git worktree")
                .request_schema(json!({
                    "type": "object",
                    "required": ["repo_path", "worktree_path", "branch"],
                    "properties": {
                        "repo_path": {"type": "string", "description": "Repository path under the workspace root"},
                        "worktree_path": {"type": "string", "description": "New worktree path under the worktree root"},
                        "branch": {"type": "string"},
                        "base_ref": {"type": ["string", "null"]}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["path", "branch", "created"],
                    "properties": {
                        "path": {"type": "string"},
                        "branch": {"type": "string"},
                        "head": {"type": ["string", "null"]},
                        "created": {"type": "boolean"}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true),
        )
        .function(
            FunctionSpec::new("git.worktree::status", CapabilityRisk::Read)
                .description("Read porcelain status for a worktree")
                .request_schema(json!({
                    "type": "object",
                    "required": ["worktree_path"],
                    "properties": {
                        "worktree_path": {"type": "string", "description": "Worktree path under the worktree root"}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["clean", "entries"],
                    "properties": {
                        "clean": {"type": "boolean"},
                        "branch": {"type": ["string", "null"]},
                        "entries": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["path", "status"],
                                "properties": {
                                    "path": {"type": "string"},
                                    "status": {"type": "string"}
                                },
                                "additionalProperties": false
                            }
                        }
                    },
                    "additionalProperties": false
                }))
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new("git.worktree::remove", CapabilityRisk::Write)
                .description("Remove a git worktree")
                .request_schema(json!({
                    "type": "object",
                    "required": ["worktree_path"],
                    "properties": {
                        "worktree_path": {"type": "string", "description": "Worktree path under the worktree root"},
                        "force": {"type": "boolean", "default": false}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["path", "removed"],
                    "properties": {
                        "path": {"type": "string"},
                        "removed": {"type": "boolean"}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true),
        )
        .sandbox(
            SandboxProfile::locked_down()
                .allow_workspace_read(WORKSPACE_ROOT)
                .allow_workspace_read(WORKTREE_ROOT)
                .allow_workspace_write(WORKTREE_ROOT)
                .allow_command("git")
                .deny_network(),
        )
}
