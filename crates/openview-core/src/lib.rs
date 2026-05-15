use chrono::{DateTime, Duration, Utc};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

pub type WorkerId = String;
pub type FunctionId = String;
pub type AgentId = String;

#[derive(Debug, Error, PartialEq, Eq)]
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
    #[error("run is not waiting for approval: {run_id}")]
    RunNotAwaitingApproval { run_id: Uuid },
    #[error("queue task not found: {0}")]
    QueueTaskNotFound(String),
    #[error("run event sequence gap for {run_id}: expected {expected}, got {actual}")]
    RunEventSequenceGap {
        run_id: Uuid,
        expected: u64,
        actual: u64,
    },
    #[error("graph has no agents")]
    EmptyGraph,
    #[error("workspace has no tabs")]
    EmptyWorkspace,
    #[error("workspace active pane not found: {0}")]
    UnknownPane(String),
    #[error("handoff references unknown agent: {0}")]
    UnknownHandoffAgent(String),
    #[error("queue task is not available: {0}")]
    QueueTaskUnavailable(String),
    #[error("queue task has no active lease: {0}")]
    QueueTaskNotLeased(String),
    #[error("queue lease mismatch for task {task_id}: expected {expected}, got {actual}")]
    QueueLeaseMismatch {
        task_id: String,
        expected: String,
        actual: String,
    },
    #[error("queue lease expired for task {task_id}: lease {lease_id} expired at {expired_at}")]
    QueueLeaseExpired {
        task_id: String,
        lease_id: String,
        expired_at: DateTime<Utc>,
    },
    #[error(
        "workflow dependency not found in {workflow_id}: node {node_id} depends on {dependency_id}"
    )]
    UnknownWorkflowDependency {
        workflow_id: String,
        node_id: String,
        dependency_id: String,
    },
    #[error("workflow contains a dependency cycle: {workflow_id}")]
    WorkflowCycle { workflow_id: String },
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

impl NetworkPolicy {
    pub fn can_access_host(&self, host: impl AsRef<str>) -> bool {
        if !self.allow {
            return false;
        }
        let Some(host) = normalize_host_name(host.as_ref()) else {
            return false;
        };
        self.hosts
            .iter()
            .filter_map(|allowed| normalize_host_name(allowed))
            .any(|allowed| allowed == host)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesystemPolicy {
    pub read_roots: IndexSet<String>,
    pub write_roots: IndexSet<String>,
    pub deny_roots: IndexSet<String>,
}

impl FilesystemPolicy {
    pub fn can_read_path(&self, path: impl AsRef<str>) -> bool {
        root_set_allows_path(&self.read_roots, &self.deny_roots, path.as_ref())
    }

    pub fn can_write_path(&self, path: impl AsRef<str>) -> bool {
        root_set_allows_path(&self.write_roots, &self.deny_roots, path.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessPolicy {
    pub allow_exec: bool,
    pub allowed_commands: IndexSet<String>,
}

impl ProcessPolicy {
    pub fn can_execute_command(&self, command: impl AsRef<str>) -> bool {
        if !self.allow_exec {
            return false;
        }
        let command = command.as_ref();
        is_command_name(command) && self.allowed_commands.contains(command)
    }
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

    pub fn allow_secret_name(mut self, name: impl Into<String>) -> Self {
        self.secret_names.insert(name.into());
        self
    }

    pub fn deny_network(mut self) -> Self {
        self.network.allow = false;
        self.network.hosts.clear();
        self
    }

    pub fn can_read_path(&self, path: impl AsRef<str>) -> bool {
        self.filesystem.can_read_path(path)
    }

    pub fn can_write_path(&self, path: impl AsRef<str>) -> bool {
        self.filesystem.can_write_path(path)
    }

    pub fn can_execute_command(&self, command: impl AsRef<str>) -> bool {
        self.process.can_execute_command(command)
    }

    pub fn can_access_network_host(&self, host: impl AsRef<str>) -> bool {
        self.network.can_access_host(host)
    }

    pub fn can_reference_secret_name(&self, name: impl AsRef<str>) -> bool {
        let name = name.as_ref();
        is_secret_name_only(name) && self.secret_names.contains(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedPath {
    absolute: bool,
    parts: Vec<String>,
}

impl NormalizedPath {
    fn specificity(&self) -> usize {
        self.parts.len()
    }

    fn contains(&self, path: &Self) -> bool {
        self.absolute == path.absolute
            && self.parts.len() <= path.parts.len()
            && self
                .parts
                .iter()
                .zip(path.parts.iter())
                .all(|(root_part, path_part)| root_part == path_part)
    }
}

fn root_set_allows_path(
    allow_roots: &IndexSet<String>,
    deny_roots: &IndexSet<String>,
    path: &str,
) -> bool {
    let Some(path) = normalize_path(path, true) else {
        return false;
    };
    let allow_specificity = best_root_specificity(allow_roots, &path);
    let Some(allow_specificity) = allow_specificity else {
        return false;
    };
    let deny_specificity = best_root_specificity(deny_roots, &path);

    deny_specificity
        .map(|specificity| specificity < allow_specificity)
        .unwrap_or(true)
}

fn best_root_specificity(roots: &IndexSet<String>, path: &NormalizedPath) -> Option<usize> {
    roots
        .iter()
        .filter_map(|root| normalize_path(root, false))
        .filter(|root| root.contains(path))
        .map(|root| root.specificity())
        .max()
}

fn normalize_path(input: &str, allow_parent_segments: bool) -> Option<NormalizedPath> {
    if input.is_empty() || input.contains('\0') {
        return None;
    }
    if !allow_parent_segments && input.split('/').any(|part| part == "..") {
        return None;
    }

    let absolute = input.starts_with('/');
    let mut parts = Vec::new();
    for part in input.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part.to_string()),
        }
    }

    if !absolute && parts.is_empty() {
        return None;
    }
    Some(NormalizedPath { absolute, parts })
}

fn is_command_name(command: &str) -> bool {
    !command.is_empty() && !command.contains('\0') && !command.chars().any(char::is_whitespace)
}

fn normalize_host_name(host: &str) -> Option<String> {
    if host.is_empty() || host.contains('\0') || host.chars().any(char::is_whitespace) {
        return None;
    }
    let host = host.strip_suffix('.').unwrap_or(host);
    if host.is_empty()
        || host
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':' | '@' | '?' | '#'))
    {
        return None;
    }
    Some(host.to_ascii_lowercase())
}

fn is_secret_name_only(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('=')
        && !name.contains('\0')
        && !name.chars().any(char::is_whitespace)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeKind {
    Action,
    Fanout,
    Join,
    Approval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub function_id: FunctionId,
    pub dependencies: Vec<String>,
    pub kind: WorkflowNodeKind,
}

impl WorkflowNode {
    pub fn action(id: impl Into<String>, function_id: impl Into<String>) -> Self {
        Self::new(id, function_id, WorkflowNodeKind::Action)
    }

    pub fn fanout(id: impl Into<String>, function_id: impl Into<String>) -> Self {
        Self::new(id, function_id, WorkflowNodeKind::Fanout)
    }

    pub fn join(id: impl Into<String>, function_id: impl Into<String>) -> Self {
        Self::new(id, function_id, WorkflowNodeKind::Join)
    }

    pub fn approval(id: impl Into<String>, function_id: impl Into<String>) -> Self {
        Self::new(id, function_id, WorkflowNodeKind::Approval)
    }

    pub fn depends_on(mut self, dependency_id: impl Into<String>) -> Self {
        self.dependencies.push(dependency_id.into());
        self
    }

    fn new(id: impl Into<String>, function_id: impl Into<String>, kind: WorkflowNodeKind) -> Self {
        Self {
            id: id.into(),
            function_id: function_id.into(),
            dependencies: Vec::new(),
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub version: String,
    pub name: String,
    pub input_schema: Value,
    pub nodes: Vec<WorkflowNode>,
}

impl WorkflowDefinition {
    pub fn new(id: impl Into<String>, version: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            name: name.into(),
            input_schema: json!({"type":"object"}),
            nodes: Vec::new(),
        }
    }

    pub fn input_schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }

    pub fn node(mut self, node: WorkflowNode) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn validate(&self) -> Result<(), OpenViewError> {
        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<IndexSet<_>>();

        for node in &self.nodes {
            for dependency_id in &node.dependencies {
                if !node_ids.contains(dependency_id) {
                    return Err(OpenViewError::UnknownWorkflowDependency {
                        workflow_id: self.id.clone(),
                        node_id: node.id.clone(),
                        dependency_id: dependency_id.clone(),
                    });
                }
            }
        }

        self.execution_plan().map(|_| ())
    }

    pub fn execution_plan(&self) -> Result<Vec<String>, OpenViewError> {
        let mut nodes_by_id = IndexMap::new();
        for node in &self.nodes {
            nodes_by_id.entry(node.id.clone()).or_insert(node);
        }

        for node in &self.nodes {
            for dependency_id in &node.dependencies {
                if !nodes_by_id.contains_key(dependency_id) {
                    return Err(OpenViewError::UnknownWorkflowDependency {
                        workflow_id: self.id.clone(),
                        node_id: node.id.clone(),
                        dependency_id: dependency_id.clone(),
                    });
                }
            }
        }

        let mut pending_dependencies = IndexMap::<String, usize>::new();
        let mut dependents = IndexMap::<String, Vec<String>>::new();
        for node in nodes_by_id.values() {
            pending_dependencies.insert(node.id.clone(), node.dependencies.len());
            for dependency_id in &node.dependencies {
                dependents
                    .entry(dependency_id.clone())
                    .or_default()
                    .push(node.id.clone());
            }
        }

        let mut ready = pending_dependencies
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(node_id, _)| node_id.clone())
            .collect::<Vec<_>>();
        let mut plan = Vec::with_capacity(nodes_by_id.len());

        while !ready.is_empty() {
            ready.sort();
            let node_id = ready.remove(0);
            if !pending_dependencies.contains_key(&node_id) {
                continue;
            }
            pending_dependencies.shift_remove(&node_id);
            plan.push(node_id.clone());

            if let Some(next_nodes) = dependents.get(&node_id) {
                for next_node_id in next_nodes {
                    let Some(count) = pending_dependencies.get_mut(next_node_id) else {
                        continue;
                    };
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.push(next_node_id.clone());
                    }
                }
            }
        }

        if plan.len() == nodes_by_id.len() {
            Ok(plan)
        } else {
            Err(OpenViewError::WorkflowCycle {
                workflow_id: self.id.clone(),
            })
        }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestMetadata {
    pub number: u64,
    pub head_branch: String,
    pub base_branch: String,
    pub title: Option<String>,
    pub author: Option<String>,
}

impl PullRequestMetadata {
    pub fn new(
        number: u64,
        head_branch: impl Into<String>,
        base_branch: impl Into<String>,
    ) -> Self {
        Self {
            number,
            head_branch: head_branch.into(),
            base_branch: base_branch.into(),
            title: None,
            author: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffReviewComment {
    pub path: String,
    pub line: u32,
    pub body: String,
    pub author: Option<String>,
    pub resolved: bool,
}

impl DiffReviewComment {
    pub fn new(path: impl Into<String>, line: u32, body: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line,
            body: body.into(),
            author: None,
            resolved: false,
        }
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn resolved(mut self) -> Self {
        self.resolved = true;
        self
    }

    pub fn unresolved(mut self) -> Self {
        self.resolved = false;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckConclusion {
    Pending,
    Passed,
    Failed,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRunStatus {
    pub name: String,
    pub conclusion: CheckConclusion,
}

impl CheckRunStatus {
    pub fn new(name: impl Into<String>, conclusion: CheckConclusion) -> Self {
        Self {
            name: name.into(),
            conclusion,
        }
    }

    pub fn pending(name: impl Into<String>) -> Self {
        Self::new(name, CheckConclusion::Pending)
    }

    pub fn passed(name: impl Into<String>) -> Self {
        Self::new(name, CheckConclusion::Passed)
    }

    pub fn failed(name: impl Into<String>) -> Self {
        Self::new(name, CheckConclusion::Failed)
    }

    pub fn blocks_merge(&self) -> bool {
        matches!(
            self.conclusion,
            CheckConclusion::Pending | CheckConclusion::Failed
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoBlocker {
    pub id: String,
    pub description: String,
    pub resolved: bool,
}

impl TodoBlocker {
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            resolved: false,
        }
    }

    pub fn resolved(mut self) -> Self {
        self.resolved = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeReadiness {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewWorkflow {
    pub pr: PullRequestMetadata,
    pub review_comments: Vec<DiffReviewComment>,
    pub checks: Vec<CheckRunStatus>,
    pub todo_blockers: Vec<TodoBlocker>,
}

impl ReviewWorkflow {
    pub fn new(pr: PullRequestMetadata) -> Self {
        Self {
            pr,
            review_comments: Vec::new(),
            checks: Vec::new(),
            todo_blockers: Vec::new(),
        }
    }

    pub fn review_comment(mut self, comment: DiffReviewComment) -> Self {
        self.review_comments.push(comment);
        self
    }

    pub fn check_run(mut self, check: CheckRunStatus) -> Self {
        self.checks.push(check);
        self
    }

    pub fn todo_blocker(mut self, blocker: TodoBlocker) -> Self {
        self.todo_blockers.push(blocker);
        self
    }

    pub fn unresolved_review_comments(&self) -> impl Iterator<Item = &DiffReviewComment> {
        self.review_comments
            .iter()
            .filter(|comment| !comment.resolved)
    }

    pub fn failing_checks(&self) -> impl Iterator<Item = &CheckRunStatus> {
        self.checks.iter().filter(|check| check.blocks_merge())
    }

    pub fn open_todo_blockers(&self) -> impl Iterator<Item = &TodoBlocker> {
        self.todo_blockers
            .iter()
            .filter(|blocker| !blocker.resolved)
    }

    pub fn merge_readiness(&self) -> MergeReadiness {
        if self.unresolved_review_comments().next().is_none()
            && self.failing_checks().next().is_none()
            && self.open_todo_blockers().next().is_none()
        {
            MergeReadiness::Ready
        } else {
            MergeReadiness::Blocked
        }
    }

    pub fn readiness_summary(&self) -> String {
        if self.merge_readiness() == MergeReadiness::Ready {
            return "ready: all review workflow contracts satisfied".to_string();
        }

        let unresolved_comments = self.unresolved_review_comments().count();
        let failing_checks = self.failing_checks().count();
        let open_todos = self.open_todo_blockers().count();
        format!(
            "blocked: {}, {}, {}",
            pluralize(unresolved_comments, "unresolved review comment"),
            pluralize(failing_checks, "failing check"),
            pluralize(open_todos, "open todo blocker")
        )
    }
}

fn pluralize(count: usize, label: &str) -> String {
    if count == 1 {
        format!("{count} {label}")
    } else {
        format!("{count} {label}s")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueTaskStatus {
    Available,
    Leased,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueEventKind {
    Enqueued,
    Leased,
    Heartbeated,
    Completed,
    Failed,
    Requeued,
    LeaseExpired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueLease {
    pub id: String,
    pub worker_id: WorkerId,
    pub leased_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl QueueLease {
    pub fn new(
        id: impl Into<String>,
        worker_id: impl Into<String>,
        leased_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            worker_id: worker_id.into(),
            leased_at,
            expires_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEvent {
    pub sequence: u64,
    pub task_id: String,
    pub kind: QueueEventKind,
    pub at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<WorkerId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueTask {
    pub id: String,
    pub queue: String,
    pub payload: Value,
    pub status: QueueTaskStatus,
    pub retry_count: u32,
    pub visible_at: DateTime<Utc>,
    #[serde(default)]
    pub lease: Option<QueueLease>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_error: Option<String>,
    pub events: Vec<QueueEvent>,
}

impl QueueTask {
    pub fn new(
        id: impl Into<String>,
        queue: impl Into<String>,
        payload: Value,
        visible_at: DateTime<Utc>,
    ) -> Self {
        let id = id.into();
        let mut task = Self {
            id,
            queue: queue.into(),
            payload,
            status: QueueTaskStatus::Available,
            retry_count: 0,
            visible_at,
            lease: None,
            completed_at: None,
            last_error: None,
            events: Vec::new(),
        };
        task.emit(QueueEventKind::Enqueued, visible_at, None, None, None);
        task
    }

    pub fn is_visible_at(&self, at: DateTime<Utc>) -> bool {
        matches!(
            self.status,
            QueueTaskStatus::Available | QueueTaskStatus::Leased
        ) && self.visible_at <= at
    }

    pub fn is_available_at(&self, at: DateTime<Utc>) -> bool {
        self.status == QueueTaskStatus::Available && self.visible_at <= at
    }

    pub fn has_active_lease_at(&self, at: DateTime<Utc>) -> bool {
        self.active_lease_at(at).is_some()
    }

    pub fn lease(
        &mut self,
        lease_id: impl Into<String>,
        worker_id: impl Into<String>,
        at: DateTime<Utc>,
        visibility_timeout: Duration,
    ) -> Result<QueueLease, OpenViewError> {
        if self.status != QueueTaskStatus::Available || self.visible_at > at {
            return Err(OpenViewError::QueueTaskUnavailable(self.id.clone()));
        }

        let lease = QueueLease::new(lease_id, worker_id, at, at + visibility_timeout);
        self.status = QueueTaskStatus::Leased;
        self.visible_at = lease.expires_at;
        self.lease = Some(lease.clone());
        self.emit(
            QueueEventKind::Leased,
            at,
            Some(lease.id.clone()),
            Some(lease.worker_id.clone()),
            None,
        );
        Ok(lease)
    }

    pub fn ack(
        &mut self,
        lease_id: impl AsRef<str>,
        at: DateTime<Utc>,
    ) -> Result<(), OpenViewError> {
        self.complete(lease_id, at)
    }

    pub fn complete(
        &mut self,
        lease_id: impl AsRef<str>,
        at: DateTime<Utc>,
    ) -> Result<(), OpenViewError> {
        let lease = self.require_active_lease(lease_id.as_ref(), at)?.clone();
        self.status = QueueTaskStatus::Completed;
        self.completed_at = Some(at);
        self.lease = None;
        self.emit(
            QueueEventKind::Completed,
            at,
            Some(lease.id),
            Some(lease.worker_id),
            None,
        );
        Ok(())
    }

    pub fn fail(
        &mut self,
        lease_id: impl AsRef<str>,
        reason: impl Into<String>,
        at: DateTime<Utc>,
        max_retries: u32,
        retry_delay: Duration,
    ) -> Result<(), OpenViewError> {
        let lease = self.require_active_lease(lease_id.as_ref(), at)?.clone();
        let reason = reason.into();
        self.retry_count = self.retry_count.saturating_add(1);
        self.last_error = Some(reason.clone());
        self.lease = None;
        self.emit(
            QueueEventKind::Failed,
            at,
            Some(lease.id.clone()),
            Some(lease.worker_id.clone()),
            Some(reason),
        );

        if self.retry_count < max_retries {
            self.status = QueueTaskStatus::Available;
            self.visible_at = at + retry_delay;
            self.emit(
                QueueEventKind::Requeued,
                at,
                Some(lease.id),
                Some(lease.worker_id),
                None,
            );
        } else {
            self.status = QueueTaskStatus::Failed;
            self.visible_at = at;
        }
        Ok(())
    }

    pub fn requeue(
        &mut self,
        lease_id: impl AsRef<str>,
        at: DateTime<Utc>,
        delay: Duration,
    ) -> Result<(), OpenViewError> {
        let lease = self.require_active_lease(lease_id.as_ref(), at)?.clone();
        self.status = QueueTaskStatus::Available;
        self.visible_at = at + delay;
        self.lease = None;
        self.emit(
            QueueEventKind::Requeued,
            at,
            Some(lease.id),
            Some(lease.worker_id),
            None,
        );
        Ok(())
    }

    pub fn heartbeat(
        &mut self,
        lease_id: impl AsRef<str>,
        at: DateTime<Utc>,
        visibility_timeout: Duration,
    ) -> Result<QueueLease, OpenViewError> {
        let lease = self.require_active_lease(lease_id.as_ref(), at)?.clone();
        let lease = QueueLease::new(
            lease.id,
            lease.worker_id,
            lease.leased_at,
            at + visibility_timeout,
        );
        self.visible_at = lease.expires_at;
        self.lease = Some(lease.clone());
        self.emit(
            QueueEventKind::Heartbeated,
            at,
            Some(lease.id.clone()),
            Some(lease.worker_id.clone()),
            None,
        );
        Ok(lease)
    }

    pub fn release_expired_lease(&mut self, at: DateTime<Utc>) -> bool {
        if self.status != QueueTaskStatus::Leased {
            return false;
        }
        let Some(active_lease) = self.lease.as_ref() else {
            return false;
        };
        if active_lease.expires_at > at {
            return false;
        }
        let lease = self.lease.take();
        self.status = QueueTaskStatus::Available;
        self.emit(
            QueueEventKind::LeaseExpired,
            at,
            lease.as_ref().map(|lease| lease.id.clone()),
            lease.map(|lease| lease.worker_id),
            None,
        );
        true
    }

    fn active_lease_at(&self, at: DateTime<Utc>) -> Option<&QueueLease> {
        if self.status != QueueTaskStatus::Leased {
            return None;
        }
        let lease = self.lease.as_ref()?;
        if lease.expires_at <= at {
            return None;
        }
        Some(lease)
    }

    fn require_active_lease(
        &self,
        actual: &str,
        at: DateTime<Utc>,
    ) -> Result<&QueueLease, OpenViewError> {
        let lease = self.require_current_lease(actual)?;
        if lease.expires_at <= at {
            return Err(OpenViewError::QueueLeaseExpired {
                task_id: self.id.clone(),
                lease_id: lease.id.clone(),
                expired_at: lease.expires_at,
            });
        }
        Ok(lease)
    }

    fn require_current_lease(&self, actual: &str) -> Result<&QueueLease, OpenViewError> {
        let lease = self
            .lease
            .as_ref()
            .ok_or_else(|| OpenViewError::QueueTaskNotLeased(self.id.clone()))?;
        if lease.id != actual {
            return Err(OpenViewError::QueueLeaseMismatch {
                task_id: self.id.clone(),
                expected: lease.id.clone(),
                actual: actual.to_string(),
            });
        }
        Ok(lease)
    }

    fn emit(
        &mut self,
        kind: QueueEventKind,
        at: DateTime<Utc>,
        lease_id: Option<String>,
        worker_id: Option<WorkerId>,
        message: Option<String>,
    ) {
        let sequence = self.events.len() as u64 + 1;
        self.events.push(QueueEvent {
            sequence,
            task_id: self.id.clone(),
            kind,
            at,
            lease_id,
            worker_id,
            message,
        });
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerRuntimeFailure {
    pub reason: String,
    pub failed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRuntimeEventKind {
    Started,
    Heartbeat,
    Stopped,
    Failed,
    Restarted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerRuntimeEvent {
    pub sequence: u64,
    pub worker_id: WorkerId,
    pub kind: WorkerRuntimeEventKind,
    pub state: WorkerHealthState,
    pub at: DateTime<Utc>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerRuntimeSpec {
    pub worker_id: WorkerId,
    pub binary: String,
    pub args: Vec<String>,
    pub env_keys: Vec<String>,
    pub working_directory: Option<String>,
    pub health_state: WorkerHealthState,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_metadata: Option<Value>,
    #[serde(default)]
    pub stopped_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_failure: Option<WorkerRuntimeFailure>,
    #[serde(default)]
    pub restart_count: u32,
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
            started_at: None,
            last_heartbeat_at: None,
            last_heartbeat_metadata: None,
            stopped_at: None,
            last_failure: None,
            restart_count: 0,
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

    pub fn start(mut self) -> Self {
        self.mark_started(Utc::now());
        self
    }

    pub fn heartbeat(mut self) -> Self {
        self.mark_heartbeat(Utc::now());
        self
    }

    pub fn ready(mut self) -> Self {
        self.mark_heartbeat(Utc::now());
        self
    }

    pub fn stop(mut self) -> Self {
        self.mark_stopped(Utc::now());
        self
    }

    pub fn fail(mut self, reason: impl Into<String>) -> Self {
        self.mark_failed(reason, Utc::now());
        self
    }

    pub fn restart(mut self) -> Self {
        self.restart_count = self.restart_count.saturating_add(1);
        self.mark_started(Utc::now());
        self
    }

    fn mark_started(&mut self, at: DateTime<Utc>) {
        self.health_state = WorkerHealthState::Starting;
        self.started_at = Some(at);
        self.stopped_at = None;
    }

    fn mark_heartbeat(&mut self, at: DateTime<Utc>) {
        if self.started_at.is_none() {
            self.started_at = Some(at);
        }
        self.health_state = WorkerHealthState::Ready;
        self.last_heartbeat_at = Some(at);
        self.stopped_at = None;
    }

    fn mark_heartbeat_with_metadata(&mut self, at: DateTime<Utc>, metadata: Option<Value>) {
        self.mark_heartbeat(at);
        self.last_heartbeat_metadata = metadata;
    }

    fn mark_stopped(&mut self, at: DateTime<Utc>) {
        self.health_state = WorkerHealthState::Stopped;
        self.stopped_at = Some(at);
    }

    fn mark_failed(&mut self, reason: impl Into<String>, at: DateTime<Utc>) {
        self.health_state = WorkerHealthState::Failed;
        self.last_failure = Some(WorkerRuntimeFailure {
            reason: reason.into(),
            failed_at: at,
        });
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerRuntimeSupervisor {
    runtimes: IndexMap<WorkerId, WorkerRuntimeSpec>,
    events: Vec<WorkerRuntimeEvent>,
}

impl WorkerRuntimeSupervisor {
    pub fn register(&mut self, runtime: WorkerRuntimeSpec) -> Result<(), OpenViewError> {
        if self.runtimes.contains_key(&runtime.worker_id) {
            return Err(OpenViewError::DuplicateWorker(runtime.worker_id));
        }
        self.runtimes.insert(runtime.worker_id.clone(), runtime);
        Ok(())
    }

    pub fn runtime(&self, worker_id: &str) -> Option<&WorkerRuntimeSpec> {
        self.runtimes.get(worker_id)
    }

    pub fn runtimes(&self) -> impl Iterator<Item = &WorkerRuntimeSpec> {
        self.runtimes.values()
    }

    pub fn events(&self) -> &[WorkerRuntimeEvent] {
        &self.events
    }

    pub fn start_worker(
        &mut self,
        worker_id: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        self.transition(
            worker_id,
            WorkerRuntimeEventKind::Started,
            None,
            |runtime, at| runtime.mark_started(at),
        )
    }

    pub fn heartbeat_worker(
        &mut self,
        worker_id: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        self.transition(
            worker_id,
            WorkerRuntimeEventKind::Heartbeat,
            None,
            |runtime, at| runtime.mark_heartbeat(at),
        )
    }

    pub fn stop_worker(
        &mut self,
        worker_id: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        self.transition(
            worker_id,
            WorkerRuntimeEventKind::Stopped,
            None,
            |runtime, at| runtime.mark_stopped(at),
        )
    }

    pub fn fail_worker(
        &mut self,
        worker_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        let reason = reason.into();
        self.transition(
            worker_id,
            WorkerRuntimeEventKind::Failed,
            Some(reason.clone()),
            |runtime, at| runtime.mark_failed(reason, at),
        )
    }

    pub fn restart_worker(
        &mut self,
        worker_id: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        self.transition(
            worker_id,
            WorkerRuntimeEventKind::Restarted,
            None,
            |runtime, at| {
                runtime.restart_count = runtime.restart_count.saturating_add(1);
                runtime.mark_started(at);
            },
        )
    }

    fn transition<F>(
        &mut self,
        worker_id: impl Into<String>,
        kind: WorkerRuntimeEventKind,
        reason: Option<String>,
        update: F,
    ) -> Result<WorkerRuntimeSpec, OpenViewError>
    where
        F: FnOnce(&mut WorkerRuntimeSpec, DateTime<Utc>),
    {
        let worker_id = worker_id.into();
        let at = Utc::now();
        let runtime = self
            .runtimes
            .get_mut(&worker_id)
            .ok_or_else(|| OpenViewError::WorkerNotFound(worker_id.clone()))?;
        update(runtime, at);
        let state = runtime.health_state;
        let snapshot = runtime.clone();
        let sequence = self.events.len() as u64 + 1;
        self.events.push(WorkerRuntimeEvent {
            sequence,
            worker_id,
            kind,
            state,
            at,
            reason,
        });
        Ok(snapshot)
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
pub struct RegisterWorkflowDefinitionRequest {
    pub definition: WorkflowDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterWorkflowDefinitionResponse {
    pub definition_id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnqueueTaskRequest {
    pub task_id: String,
    pub queue: String,
    pub payload: Value,
    pub visible_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnqueueTaskResponse {
    pub task: QueueTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollWorkRequest {
    pub worker_id: WorkerId,
    pub lease_id: String,
    pub now: DateTime<Utc>,
    pub visibility_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PollWorkResponse {
    pub task: Option<QueueTask>,
    pub lease: Option<QueueLease>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueConcurrencyPolicyKind {
    LocallyEnforced,
    DelegatedToIiiQueue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueConcurrencySnapshotRequest {
    pub queue: String,
    pub requested_max_concurrency: usize,
    pub now: DateTime<Utc>,
    pub policy_kind: QueueConcurrencyPolicyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueConcurrencySnapshotResponse {
    pub queue: String,
    pub requested_max_concurrency: usize,
    pub active_lease_count: usize,
    pub available_task_count: usize,
    pub blocked_by_limit: bool,
    pub policy_kind: QueueConcurrencyPolicyKind,
    pub locally_enforced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatTaskRequest {
    pub task_id: String,
    pub lease_id: String,
    pub heartbeat_at: DateTime<Utc>,
    pub visibility_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatTaskResponse {
    pub task: QueueTask,
    pub lease: QueueLease,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatWorkerLeaseRequest {
    pub worker_id: WorkerId,
    pub queue: String,
    pub heartbeat_at: DateTime<Utc>,
    pub visibility_timeout: Duration,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatWorkerLeaseResponse {
    pub runtime: WorkerRuntimeSpec,
    pub tasks: Vec<QueueTask>,
    pub leases: Vec<QueueLease>,
}

pub type HeartbeatWorkerLeasesRequest = HeartbeatWorkerLeaseRequest;
pub type HeartbeatWorkerLeasesResponse = HeartbeatWorkerLeaseResponse;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverExpiredLeasesRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<String>,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoverExpiredLeasesResponse {
    pub tasks: Vec<QueueTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompleteTaskRequest {
    pub task_id: String,
    pub lease_id: String,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompleteTaskResponse {
    pub task: QueueTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailTaskRequest {
    pub task_id: String,
    pub lease_id: String,
    pub reason: String,
    pub failed_at: DateTime<Utc>,
    pub max_retries: u32,
    pub retry_delay: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailTaskResponse {
    pub task: QueueTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveApprovalRequest {
    pub run_id: Uuid,
    pub approval_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproveApprovalResponse {
    pub run: RunRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectApprovalRequest {
    pub run_id: Uuid,
    pub approval_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectApprovalResponse {
    pub run: RunRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelRunRequest {
    pub run_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelRunResponse {
    pub run: RunRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadEventsRequest {
    pub cursor: EventCursor,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: String,
    pub run_id: Uuid,
    pub uri: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
    pub metadata: IndexMap<String, String>,
}

impl ArtifactRecord {
    pub fn new(
        id: impl Into<String>,
        run_id: Uuid,
        uri: impl Into<String>,
        content_type: impl Into<String>,
        size_bytes: u64,
        sha256: impl Into<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            run_id,
            uri: uri.into(),
            content_type: content_type.into(),
            size_bytes,
            sha256: sha256.into(),
            created_at,
            metadata: IndexMap::new(),
        }
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueTaskSummary {
    pub id: String,
    pub queue: String,
    pub status: QueueTaskStatus,
    pub retry_count: u32,
    pub visible_at: DateTime<Utc>,
    pub lease_id: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub event_count: usize,
}

impl QueueTaskSummary {
    fn from_task(task: &QueueTask) -> Self {
        Self {
            id: task.id.clone(),
            queue: task.queue.clone(),
            status: task.status,
            retry_count: task.retry_count,
            visible_at: task.visible_at,
            lease_id: task.lease.as_ref().map(|lease| lease.id.clone()),
            completed_at: task.completed_at,
            last_error: task.last_error.clone(),
            event_count: task.events.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub run: RunRecord,
    pub events: Vec<RunEvent>,
    pub approvals: Vec<ApprovalRecord>,
    pub worker_runtimes: Vec<WorkerRuntimeSpec>,
    pub queue_tasks: Vec<QueueTaskSummary>,
    pub artifacts: Vec<ArtifactRecord>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenViewStoreSnapshot {
    pub workflow_definitions: IndexMap<String, WorkflowDefinition>,
    pub runs: IndexMap<Uuid, RunRecord>,
    pub run_events: IndexMap<Uuid, Vec<RunEvent>>,
    pub queue_tasks: IndexMap<String, QueueTask>,
    pub worker_runtimes: IndexMap<WorkerId, WorkerRuntimeSpec>,
    pub artifacts: IndexMap<String, ArtifactRecord>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalRunStore {
    workflow_definitions: IndexMap<String, WorkflowDefinition>,
    runs: IndexMap<Uuid, RunRecord>,
    run_events: IndexMap<Uuid, Vec<RunEvent>>,
    queue_tasks: IndexMap<String, QueueTask>,
    worker_runtimes: IndexMap<WorkerId, WorkerRuntimeSpec>,
    artifacts: IndexMap<String, ArtifactRecord>,
}

impl LocalRunStore {
    pub fn from_snapshot(snapshot: OpenViewStoreSnapshot) -> Result<Self, OpenViewError> {
        for (run_id, events) in &snapshot.run_events {
            validate_run_event_sequence(*run_id, events)?;
        }

        Ok(Self {
            workflow_definitions: snapshot.workflow_definitions,
            runs: snapshot.runs,
            run_events: snapshot.run_events,
            queue_tasks: snapshot.queue_tasks,
            worker_runtimes: snapshot.worker_runtimes,
            artifacts: snapshot.artifacts,
        })
    }

    pub fn snapshot(&self) -> OpenViewStoreSnapshot {
        OpenViewStoreSnapshot {
            workflow_definitions: self.workflow_definitions.clone(),
            runs: self.runs.clone(),
            run_events: self.run_events.clone(),
            queue_tasks: self.queue_tasks.clone(),
            worker_runtimes: self.worker_runtimes.clone(),
            artifacts: self.artifacts.clone(),
        }
    }

    pub fn persist_workflow_definition(
        &mut self,
        definition: WorkflowDefinition,
    ) -> Result<(), OpenViewError> {
        definition.validate()?;
        self.workflow_definitions
            .insert(definition.id.clone(), definition);
        Ok(())
    }

    pub fn workflow_definition(&self, id: &str) -> Option<&WorkflowDefinition> {
        self.workflow_definitions.get(id)
    }

    pub fn persist_run(&mut self, mut run: RunRecord) -> Result<(), OpenViewError> {
        validate_run_event_sequence(run.id, &run.events)?;
        let events = std::mem::take(&mut run.events);
        if events.is_empty() {
            self.run_events.entry(run.id).or_default();
        } else {
            self.run_events.insert(run.id, events);
        }
        self.runs.insert(run.id, run);
        Ok(())
    }

    pub fn persist_event(&mut self, run_id: Uuid, event: RunEvent) -> Result<(), OpenViewError> {
        if !self.runs.contains_key(&run_id) {
            return Err(OpenViewError::RunNotFound(run_id));
        }
        let events = self.run_events.entry(run_id).or_default();
        let expected = events.len() as u64 + 1;
        if event.sequence != expected {
            return Err(OpenViewError::RunEventSequenceGap {
                run_id,
                expected,
                actual: event.sequence,
            });
        }
        events.push(event);
        Ok(())
    }

    pub fn run(&self, run_id: Uuid) -> Option<&RunRecord> {
        self.runs.get(&run_id)
    }

    pub fn replay_run_state(&self, run_id: Uuid) -> Result<RunRecord, OpenViewError> {
        let mut run = self
            .runs
            .get(&run_id)
            .cloned()
            .ok_or(OpenViewError::RunNotFound(run_id))?;
        let events = self.run_events.get(&run_id).cloned().unwrap_or_default();
        validate_run_event_sequence(run_id, &events)?;
        run.events = events;
        Ok(run)
    }

    pub fn runs_for_worker(&self, worker_id: &str) -> Vec<&RunRecord> {
        self.runs
            .values()
            .filter(|run| {
                run.workers.iter().any(|worker| worker == worker_id)
                    || run.steps.iter().any(|step| step.worker_id == worker_id)
            })
            .collect()
    }

    pub fn approvals_by_state(&self, approved: bool) -> Vec<&ApprovalRecord> {
        self.runs
            .values()
            .flat_map(|run| run.approvals.iter())
            .filter(|approval| approval.approved == approved)
            .collect()
    }

    pub fn persist_queue_task(&mut self, task: QueueTask) -> Result<(), OpenViewError> {
        validate_queue_event_sequence(&task)?;
        self.queue_tasks.insert(task.id.clone(), task);
        Ok(())
    }

    pub fn queue_task(&self, task_id: &str) -> Option<&QueueTask> {
        self.queue_tasks.get(task_id)
    }

    pub fn persist_worker_runtime(
        &mut self,
        runtime: WorkerRuntimeSpec,
    ) -> Result<(), OpenViewError> {
        self.worker_runtimes
            .insert(runtime.worker_id.clone(), runtime);
        Ok(())
    }

    pub fn worker_runtime(&self, worker_id: &str) -> Option<&WorkerRuntimeSpec> {
        self.worker_runtimes.get(worker_id)
    }

    pub fn persist_artifact(&mut self, artifact: ArtifactRecord) -> Result<(), OpenViewError> {
        if !self.runs.contains_key(&artifact.run_id) {
            return Err(OpenViewError::RunNotFound(artifact.run_id));
        }
        self.artifacts.insert(artifact.id.clone(), artifact);
        Ok(())
    }

    pub fn artifacts_for_run(&self, run_id: Uuid) -> Result<Vec<ArtifactRecord>, OpenViewError> {
        if !self.runs.contains_key(&run_id) {
            return Err(OpenViewError::RunNotFound(run_id));
        }
        let mut artifacts = self
            .artifacts
            .values()
            .filter(|artifact| artifact.run_id == run_id)
            .cloned()
            .collect::<Vec<_>>();
        artifacts.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.created_at.cmp(&right.created_at))
        });
        Ok(artifacts)
    }

    pub fn export_evidence_bundle(&self, run_id: Uuid) -> Result<EvidenceBundle, OpenViewError> {
        let run = self.replay_run_state(run_id)?;
        let mut worker_ids = run.workers.iter().cloned().collect::<IndexSet<_>>();
        worker_ids.extend(run.steps.iter().map(|step| step.worker_id.clone()));

        let mut worker_runtimes = self
            .worker_runtimes
            .values()
            .filter(|runtime| worker_ids.contains(&runtime.worker_id))
            .cloned()
            .collect::<Vec<_>>();
        worker_runtimes.sort_by(|left, right| left.worker_id.cmp(&right.worker_id));

        let mut queue_tasks = self
            .queue_tasks
            .values()
            .filter(|task| queue_task_mentions_run(task, run_id))
            .map(QueueTaskSummary::from_task)
            .collect::<Vec<_>>();
        queue_tasks.sort_by(|left, right| left.id.cmp(&right.id));

        let artifacts = self.artifacts_for_run(run_id)?;
        Ok(EvidenceBundle {
            events: run.events.clone(),
            approvals: run.approvals.clone(),
            run,
            worker_runtimes,
            queue_tasks,
            artifacts,
        })
    }
}

fn queue_task_mentions_run(task: &QueueTask, run_id: Uuid) -> bool {
    let run_id = run_id.to_string();
    task.payload
        .get("run_id")
        .and_then(Value::as_str)
        .map(|value| value == run_id)
        .unwrap_or(false)
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenViewControlApi {
    store: LocalRunStore,
}

impl OpenViewControlApi {
    pub fn new(store: LocalRunStore) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &LocalRunStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut LocalRunStore {
        &mut self.store
    }

    pub fn into_store(self) -> LocalRunStore {
        self.store
    }

    pub fn register_workflow_definition(
        &mut self,
        request: RegisterWorkflowDefinitionRequest,
    ) -> Result<RegisterWorkflowDefinitionResponse, OpenViewError> {
        let definition_id = request.definition.id.clone();
        let version = request.definition.version.clone();
        self.store.persist_workflow_definition(request.definition)?;
        Ok(RegisterWorkflowDefinitionResponse {
            definition_id,
            version,
        })
    }

    pub fn enqueue_task(
        &mut self,
        request: EnqueueTaskRequest,
    ) -> Result<EnqueueTaskResponse, OpenViewError> {
        let task = QueueTask::new(
            request.task_id,
            request.queue,
            request.payload,
            request.visible_at,
        );
        self.store.persist_queue_task(task.clone())?;
        Ok(EnqueueTaskResponse { task })
    }

    pub fn poll_work(
        &mut self,
        request: PollWorkRequest,
    ) -> Result<PollWorkResponse, OpenViewError> {
        self.poll_work_with_concurrency_limit(request, usize::MAX)
    }

    pub fn poll_work_with_concurrency_limit(
        &mut self,
        request: PollWorkRequest,
        max_concurrent_leases: usize,
    ) -> Result<PollWorkResponse, OpenViewError> {
        let queue = request.worker_id.clone();
        self.recover_expired_queue_leases(Some(&queue), request.now);

        if self.active_queue_lease_count(&queue, request.now) >= max_concurrent_leases {
            return Ok(PollWorkResponse {
                task: None,
                lease: None,
            });
        }

        let task_id = self
            .store
            .queue_tasks
            .values()
            .find(|task| task.queue == queue && task.is_available_at(request.now))
            .map(|task| task.id.clone());

        let Some(task_id) = task_id else {
            return Ok(PollWorkResponse {
                task: None,
                lease: None,
            });
        };

        let task = self
            .store
            .queue_tasks
            .get_mut(&task_id)
            .ok_or_else(|| OpenViewError::QueueTaskNotFound(task_id.clone()))?;
        let lease = task.lease(
            request.lease_id,
            request.worker_id,
            request.now,
            request.visibility_timeout,
        )?;
        let task = task.clone();
        Ok(PollWorkResponse {
            task: Some(task),
            lease: Some(lease),
        })
    }

    pub fn queue_concurrency_snapshot(
        &mut self,
        request: QueueConcurrencySnapshotRequest,
    ) -> Result<QueueConcurrencySnapshotResponse, OpenViewError> {
        self.recover_expired_queue_leases(Some(&request.queue), request.now);

        let active_lease_count = self.active_queue_lease_count(&request.queue, request.now);
        let available_task_count = self.available_queue_task_count(&request.queue, request.now);
        let locally_enforced = request.policy_kind == QueueConcurrencyPolicyKind::LocallyEnforced;
        let blocked_by_limit =
            locally_enforced && active_lease_count >= request.requested_max_concurrency;

        Ok(QueueConcurrencySnapshotResponse {
            queue: request.queue,
            requested_max_concurrency: request.requested_max_concurrency,
            active_lease_count,
            available_task_count,
            blocked_by_limit,
            policy_kind: request.policy_kind,
            locally_enforced,
        })
    }

    pub fn heartbeat_task(
        &mut self,
        request: HeartbeatTaskRequest,
    ) -> Result<HeartbeatTaskResponse, OpenViewError> {
        let task = self
            .store
            .queue_tasks
            .get_mut(&request.task_id)
            .ok_or_else(|| OpenViewError::QueueTaskNotFound(request.task_id.clone()))?;
        let lease = task.heartbeat(
            request.lease_id,
            request.heartbeat_at,
            request.visibility_timeout,
        )?;
        Ok(HeartbeatTaskResponse {
            task: task.clone(),
            lease,
        })
    }

    pub fn heartbeat_worker_leases(
        &mut self,
        request: HeartbeatWorkerLeaseRequest,
    ) -> Result<HeartbeatWorkerLeaseResponse, OpenViewError> {
        let runtime = self
            .store
            .worker_runtimes
            .get_mut(&request.worker_id)
            .ok_or_else(|| OpenViewError::WorkerNotFound(request.worker_id.clone()))?;
        runtime.mark_heartbeat_with_metadata(request.heartbeat_at, request.metadata);
        let runtime = runtime.clone();

        let mut tasks = Vec::new();
        let mut leases = Vec::new();
        for task in self.store.queue_tasks.values_mut() {
            if task.queue != request.queue {
                continue;
            }
            let lease_id = task
                .active_lease_at(request.heartbeat_at)
                .filter(|lease| lease.worker_id == request.worker_id)
                .map(|lease| lease.id.clone());
            let Some(lease_id) = lease_id else {
                continue;
            };

            let lease =
                task.heartbeat(lease_id, request.heartbeat_at, request.visibility_timeout)?;
            leases.push(lease);
            tasks.push(task.clone());
        }

        Ok(HeartbeatWorkerLeaseResponse {
            runtime,
            tasks,
            leases,
        })
    }

    pub fn recover_expired_leases(
        &mut self,
        request: RecoverExpiredLeasesRequest,
    ) -> Result<RecoverExpiredLeasesResponse, OpenViewError> {
        let tasks = self.recover_expired_queue_leases(request.queue.as_deref(), request.now);
        Ok(RecoverExpiredLeasesResponse { tasks })
    }

    pub fn complete_task(
        &mut self,
        request: CompleteTaskRequest,
    ) -> Result<CompleteTaskResponse, OpenViewError> {
        let task = self
            .store
            .queue_tasks
            .get_mut(&request.task_id)
            .ok_or_else(|| OpenViewError::QueueTaskNotFound(request.task_id.clone()))?;
        task.complete(request.lease_id, request.completed_at)?;
        Ok(CompleteTaskResponse { task: task.clone() })
    }

    pub fn fail_task(
        &mut self,
        request: FailTaskRequest,
    ) -> Result<FailTaskResponse, OpenViewError> {
        let task = self
            .store
            .queue_tasks
            .get_mut(&request.task_id)
            .ok_or_else(|| OpenViewError::QueueTaskNotFound(request.task_id.clone()))?;
        task.fail(
            request.lease_id,
            request.reason,
            request.failed_at,
            request.max_retries,
            request.retry_delay,
        )?;
        Ok(FailTaskResponse { task: task.clone() })
    }

    fn recover_expired_queue_leases(
        &mut self,
        queue: Option<&str>,
        at: DateTime<Utc>,
    ) -> Vec<QueueTask> {
        let mut recovered = Vec::new();
        for task in self.store.queue_tasks.values_mut() {
            if queue.map(|queue| task.queue != queue).unwrap_or(false) {
                continue;
            }
            if task.release_expired_lease(at) {
                recovered.push(task.clone());
            }
        }
        recovered
    }

    fn active_queue_lease_count(&self, queue: &str, at: DateTime<Utc>) -> usize {
        self.store
            .queue_tasks
            .values()
            .filter(|task| task.queue == queue && task.has_active_lease_at(at))
            .count()
    }

    fn available_queue_task_count(&self, queue: &str, at: DateTime<Utc>) -> usize {
        self.store
            .queue_tasks
            .values()
            .filter(|task| task.queue == queue && task.is_available_at(at))
            .count()
    }

    pub fn approve_approval(
        &mut self,
        request: ApproveApprovalRequest,
    ) -> Result<ApproveApprovalResponse, OpenViewError> {
        let mut run = self
            .store
            .replay_run_state(request.run_id)
            .map_err(|_| OpenViewError::ApprovalNotFound(request.approval_id))?;
        if run.phase != RunPhase::WaitingForApproval {
            return Err(OpenViewError::RunNotAwaitingApproval { run_id: run.id });
        }

        {
            let approval = run
                .approvals
                .iter_mut()
                .find(|approval| approval.id == request.approval_id)
                .ok_or(OpenViewError::ApprovalNotFound(request.approval_id))?;
            approval.approved = true;
            approval.resolved_reason = Some(request.reason.clone());
            approval.resolved_at = Some(Utc::now());
        }

        for step in &mut run.steps {
            if step.phase == RunPhase::WaitingForApproval {
                step.phase = RunPhase::Completed;
            }
        }
        Self::emit_run_event(
            &mut run,
            "approval.approved",
            json!({"approval_id": request.approval_id, "reason": request.reason}),
        );
        run.phase = RunPhase::Completed;
        let run_id_payload = run.id;
        Self::emit_run_event(&mut run, "run.completed", json!({"run_id": run_id_payload}));
        self.store.persist_run(run.clone())?;
        Ok(ApproveApprovalResponse { run })
    }

    pub fn reject_approval(
        &mut self,
        request: RejectApprovalRequest,
    ) -> Result<RejectApprovalResponse, OpenViewError> {
        let mut run = self
            .store
            .replay_run_state(request.run_id)
            .map_err(|_| OpenViewError::ApprovalNotFound(request.approval_id))?;
        if run.phase != RunPhase::WaitingForApproval {
            return Err(OpenViewError::RunNotAwaitingApproval { run_id: run.id });
        }

        {
            let approval = run
                .approvals
                .iter_mut()
                .find(|approval| approval.id == request.approval_id)
                .ok_or(OpenViewError::ApprovalNotFound(request.approval_id))?;
            approval.approved = false;
            approval.resolved_reason = Some(request.reason.clone());
            approval.resolved_at = Some(Utc::now());
        }

        for step in &mut run.steps {
            if step.phase == RunPhase::WaitingForApproval {
                step.phase = RunPhase::Failed;
            }
        }
        Self::emit_run_event(
            &mut run,
            "approval.rejected",
            json!({"approval_id": request.approval_id, "reason": request.reason}),
        );
        run.phase = RunPhase::Failed;
        let run_id_payload = run.id;
        Self::emit_run_event(&mut run, "run.failed", json!({"run_id": run_id_payload}));
        self.store.persist_run(run.clone())?;
        Ok(RejectApprovalResponse { run })
    }

    pub fn cancel_run(
        &mut self,
        request: CancelRunRequest,
    ) -> Result<CancelRunResponse, OpenViewError> {
        let mut run = self.store.replay_run_state(request.run_id)?;
        run.phase = RunPhase::Cancelled;
        for step in &mut run.steps {
            if !matches!(
                step.phase,
                RunPhase::Completed | RunPhase::Failed | RunPhase::Cancelled
            ) {
                step.phase = RunPhase::Cancelled;
            }
        }
        let run_id_payload = run.id;
        Self::emit_run_event(
            &mut run,
            "run.cancelled",
            json!({"run_id": run_id_payload, "reason": request.reason}),
        );
        self.store.persist_run(run.clone())?;
        Ok(CancelRunResponse { run })
    }

    pub fn read_events(&self, request: ReadEventsRequest) -> Result<EventPage, OpenViewError> {
        let cursor = request.cursor;
        if !self.store.runs.contains_key(&cursor.run_id) {
            return Err(OpenViewError::RunNotFound(cursor.run_id));
        }
        let all_events = self
            .store
            .run_events
            .get(&cursor.run_id)
            .cloned()
            .unwrap_or_default();
        validate_run_event_sequence(cursor.run_id, &all_events)?;
        let events = all_events
            .iter()
            .filter(|event| event.sequence > cursor.after_sequence)
            .take(cursor.limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_after_sequence = events
            .last()
            .map(|event| event.sequence)
            .unwrap_or(cursor.after_sequence);
        let has_more = all_events
            .iter()
            .any(|event| event.sequence > next_after_sequence);
        Ok(EventPage {
            run_id: cursor.run_id,
            events,
            next_after_sequence,
            has_more,
        })
    }

    fn emit_run_event(run: &mut RunRecord, kind: impl Into<String>, payload: Value) {
        let sequence = run.events.len() as u64 + 1;
        run.events.push(RunEvent {
            sequence,
            kind: kind.into(),
            at: Utc::now(),
            payload,
        });
    }
}

fn validate_run_event_sequence(run_id: Uuid, events: &[RunEvent]) -> Result<(), OpenViewError> {
    for (index, event) in events.iter().enumerate() {
        let expected = index as u64 + 1;
        if event.sequence != expected {
            return Err(OpenViewError::RunEventSequenceGap {
                run_id,
                expected,
                actual: event.sequence,
            });
        }
    }
    Ok(())
}

fn validate_queue_event_sequence(task: &QueueTask) -> Result<(), OpenViewError> {
    for (index, event) in task.events.iter().enumerate() {
        let expected = index as u64 + 1;
        if event.sequence != expected {
            return Err(OpenViewError::RunEventSequenceGap {
                run_id: Uuid::nil(),
                expected,
                actual: event.sequence,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct OpenViewRuntime {
    registry: WorkerRegistry,
    worker_runtimes: WorkerRuntimeSupervisor,
    runs: IndexMap<Uuid, RunRecord>,
}

impl OpenViewRuntime {
    pub fn register_worker(&mut self, manifest: WorkerManifest) -> Result<(), OpenViewError> {
        self.registry.register(manifest)
    }

    pub fn registry(&self) -> &WorkerRegistry {
        &self.registry
    }

    pub fn register_worker_runtime(
        &mut self,
        runtime: WorkerRuntimeSpec,
    ) -> Result<(), OpenViewError> {
        self.worker_runtimes.register(runtime)
    }

    pub fn worker_runtime(&self, worker_id: &str) -> Option<&WorkerRuntimeSpec> {
        self.worker_runtimes.runtime(worker_id)
    }

    pub fn worker_runtimes(&self) -> impl Iterator<Item = &WorkerRuntimeSpec> {
        self.worker_runtimes.runtimes()
    }

    pub fn worker_runtime_events(&self) -> &[WorkerRuntimeEvent] {
        self.worker_runtimes.events()
    }

    pub fn start_worker_runtime(
        &mut self,
        worker_id: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        self.worker_runtimes.start_worker(worker_id)
    }

    pub fn heartbeat_worker_runtime(
        &mut self,
        worker_id: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        self.worker_runtimes.heartbeat_worker(worker_id)
    }

    pub fn stop_worker_runtime(
        &mut self,
        worker_id: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        self.worker_runtimes.stop_worker(worker_id)
    }

    pub fn fail_worker_runtime(
        &mut self,
        worker_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        self.worker_runtimes.fail_worker(worker_id, reason)
    }

    pub fn restart_worker_runtime(
        &mut self,
        worker_id: impl Into<String>,
    ) -> Result<WorkerRuntimeSpec, OpenViewError> {
        self.worker_runtimes.restart_worker(worker_id)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IiiDurableOperation {
    SpawnTask,
    ClaimLease,
    CheckpointStepCache,
    ReadCheckpointStepCache,
    AwaitEvent,
    EmitEvent,
    EmitEventFirstWins,
    FanoutEvent,
    Sleep,
    Retry,
    Cancel,
    ListPendingApprovals,
    ResolveApproval,
    EvidenceStreamAppend,
    EvidenceStreamRead,
    HermesRunStart,
    HermesRunStartAndWait,
    SessionCreate,
    SessionAppend,
    SessionMessages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IiiQueuePrimitive {
    IiiQueuePreferred,
    DatabaseLeaseFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IiiRunMode {
    Start,
    StartAndWait,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IiiFunctionCallPlan {
    pub operation: IiiDurableOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_id: Option<FunctionId>,
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<Value>,
    pub semantics: Vec<String>,
}

impl IiiFunctionCallPlan {
    fn function(
        operation: IiiDurableOperation,
        function_id: impl Into<String>,
        payload: Value,
        semantics: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            operation,
            function_id: Some(function_id.into()),
            payload,
            action: None,
            semantics: semantics.into_iter().map(Into::into).collect(),
        }
    }

    fn queued_function(
        operation: IiiDurableOperation,
        function_id: impl Into<String>,
        payload: Value,
        queue: impl Into<String>,
        semantics: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            operation,
            function_id: Some(function_id.into()),
            payload,
            action: Some(json!({ "kind": "enqueue", "queue": queue.into() })),
            semantics: semantics.into_iter().map(Into::into).collect(),
        }
    }

    fn runtime(
        operation: IiiDurableOperation,
        payload: Value,
        semantics: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            operation,
            function_id: None,
            payload,
            action: None,
            semantics: semantics.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IiiBackendPlan {
    pub queue_primitive: IiiQueuePrimitive,
    pub planning_only: bool,
    pub calls: Vec<IiiFunctionCallPlan>,
}

impl IiiBackendPlan {
    pub fn call(&self, operation: IiiDurableOperation) -> Option<&IiiFunctionCallPlan> {
        self.calls.iter().find(|call| call.operation == operation)
    }

    pub fn function_id(&self, operation: IiiDurableOperation) -> Option<&str> {
        self.call(operation)
            .and_then(|call| call.function_id.as_deref())
    }

    pub fn action(&self, operation: IiiDurableOperation) -> Option<&Value> {
        self.call(operation).and_then(|call| call.action.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IiiBackendConfig {
    pub queue_name: String,
    pub task_name: String,
    pub idempotency_key: Option<String>,
    pub worker_id: WorkerId,
    pub session_id: String,
    pub run_goal: String,
    pub queue_available: bool,
    pub hermes_mode: IiiRunMode,
}

impl Default for IiiBackendConfig {
    fn default() -> Self {
        Self {
            queue_name: "openview-runs".to_string(),
            task_name: "openview-task".to_string(),
            idempotency_key: None,
            worker_id: "openview-worker".to_string(),
            session_id: "openview-session".to_string(),
            run_goal: "execute OpenView durable task".to_string(),
            queue_available: true,
            hermes_mode: IiiRunMode::Start,
        }
    }
}

impl IiiBackendConfig {
    pub fn queue_name(mut self, queue_name: impl Into<String>) -> Self {
        self.queue_name = queue_name.into();
        self
    }

    pub fn task_name(mut self, task_name: impl Into<String>) -> Self {
        self.task_name = task_name.into();
        self
    }

    pub fn idempotency_key(mut self, idempotency_key: impl Into<String>) -> Self {
        self.idempotency_key = Some(idempotency_key.into());
        self
    }

    pub fn worker_id(mut self, worker_id: impl Into<String>) -> Self {
        self.worker_id = worker_id.into();
        self
    }

    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }

    pub fn run_goal(mut self, run_goal: impl Into<String>) -> Self {
        self.run_goal = run_goal.into();
        self
    }

    pub fn without_iii_queue(mut self) -> Self {
        self.queue_available = false;
        self
    }

    pub fn hermes_mode(mut self, mode: IiiRunMode) -> Self {
        self.hermes_mode = mode;
        self
    }

    pub fn build_plan(&self) -> IiiBackendPlan {
        let queue_primitive = if self.queue_available {
            IiiQueuePrimitive::IiiQueuePreferred
        } else {
            IiiQueuePrimitive::DatabaseLeaseFallback
        };
        let mut calls = Vec::new();

        match queue_primitive {
            IiiQueuePrimitive::IiiQueuePreferred => calls.push(IiiFunctionCallPlan::queued_function(
                IiiDurableOperation::SpawnTask,
                self.task_name.clone(),
                json!({
                    "task": self.task_name,
                    "idempotency_key": self.idempotency_key,
                    "headers": {
                        "worker_id": self.worker_id,
                        "runtime": "openview"
                    }
                }),
                self.queue_name.clone(),
                [
                    "spawn-time deduplication is enforced by the OpenView adapter before enqueue",
                    "iii named queues use TriggerAction.Enqueue with a configured queue name",
                ],
            )),
            IiiQueuePrimitive::DatabaseLeaseFallback => calls.push(IiiFunctionCallPlan::function(
                IiiDurableOperation::SpawnTask,
                "iii-database::transaction",
                json!({
                    "db": "openview",
                    "statements": [
                        {"sql": "INSERT INTO durable_idempotency_keys (idempotency_key, task_id) VALUES (?, ?) ON CONFLICT (idempotency_key) DO NOTHING", "params": ["${idempotency_key}", "${task_id}"]},
                        {"sql": "INSERT INTO durable_tasks (task_id, queue, task_name, payload, visible_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT (task_id) DO NOTHING", "params": ["${task_id}", self.queue_name, self.task_name, "${json_payload}", "${now}"]}
                    ]
                }),
                [
                    "spawn-time deduplication returns an existing task for the same idempotency key",
                    "task payload and headers are persisted before workers can claim it",
                ],
            )),
        }

        match queue_primitive {
            IiiQueuePrimitive::IiiQueuePreferred => calls.push(IiiFunctionCallPlan::runtime(
                IiiDurableOperation::ClaimLease,
                json!({
                    "queue": self.queue_name,
                    "worker_id": self.worker_id,
                    "lease_owner": "iii-queue",
                    "delivery": "iii queue consumer invokes the target function with retry and concurrency policy"
                }),
                [
                    "iii-queue owns delivery, retry, and in-flight lease semantics for named queues",
                    "OpenView records the delivered task/run state at the adapter boundary",
                ],
            )),
            IiiQueuePrimitive::DatabaseLeaseFallback => calls.push(IiiFunctionCallPlan::function(
                IiiDurableOperation::ClaimLease,
                "iii-database::transaction",
                json!({
                    "db": "openview",
                    "lease_table": "durable_task_leases",
                    "where": "visible_at <= now and lease_expires_at < now",
                    "statements": [
                        {"sql": "UPDATE durable_tasks SET lease_owner = ?, lease_expires_at = ? WHERE queue = ? AND visible_at <= ? AND (lease_expires_at IS NULL OR lease_expires_at < ?) RETURNING task_id", "params": [self.worker_id, "${lease_expires_at}", self.queue_name, "${now}", "${now}"]}
                    ]
                }),
                [
                    "worker claims a task with a time-limited claim lease",
                    "lease fallback uses iii-database transaction when iii-queue is unavailable",
                ],
            )),
        }

        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::CheckpointStepCache,
            "iii-database::transaction",
            json!({
                "db": "openview",
                "table": "durable_step_checkpoints",
                "statements": [
                    {"sql": "INSERT INTO durable_step_checkpoints (task_id, step_name, state) VALUES (?, ?, ?) ON CONFLICT (task_id, step_name) DO NOTHING", "params": ["${task_id}", "${step_name}", "${json_state}"]},
                    {"sql": "UPDATE durable_tasks SET lease_expires_at = ? WHERE task_id = ?", "params": ["${lease_expires_at}", "${task_id}"]}
                ]
            }),
            [
                "completed steps are cached and skipped on replay",
                "checkpoint commits and lease extension must be atomic",
            ],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::ReadCheckpointStepCache,
            "iii-database::query",
            json!({
                "db": "openview",
                "table": "durable_step_checkpoints",
                "sql": "SELECT state FROM durable_step_checkpoints WHERE task_id = ? AND step_name = ?",
                "params": ["${task_id}", "${step_name}"]
            }),
            ["checkpoint cache reads prevent duplicate step side effects during replay"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::AwaitEvent,
            "stream::list",
            json!({
                "stream_name": "openview::events",
                "group_id": self.session_id,
                "event_name": "${event_name}",
                "timeout": "optional"
            }),
            ["await event resumes from stream state or timeout without live polling in the planner"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::EmitEvent,
            "stream::set",
            json!({
                "stream_name": "openview::events",
                "group_id": self.session_id,
                "item_id": "${session_id}-${sequence}",
                "data": {
                    "event_name": "${event_name}",
                    "payload": "${json_payload}"
                }
            }),
            ["event payload is appended to the iii stream for consumers and evidence"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::EmitEventFirstWins,
            "iii-database::execute",
            json!({
                "db": "openview",
                "sql": "INSERT INTO durable_events (queue, event_name, payload) VALUES (?, ?, ?) ON CONFLICT (queue, event_name) DO NOTHING",
                "params": [self.queue_name, "${event_name}", "${json_payload}"],
                "returning": ["event_name", "payload"]
            }),
            ["first emit for an event name wins and later emits are ignored"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::FanoutEvent,
            "hook-fanout::publish_collect",
            json!({
                "topic": "openview::durable_event",
                "payload": {
                    "event_name": "${event_name}",
                    "payload": "${json_payload}"
                },
                "merge_rule": "collect_all",
                "timeout_ms": 5000
            }),
            ["hook fanout wakes registered waiters after first-wins event persistence"],
        ));

        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::Sleep,
            "iii-database::execute",
            json!({
                "db": "openview",
                "sql": "UPDATE durable_tasks SET visible_at = ?, lease_owner = NULL, lease_expires_at = NULL WHERE task_id = ?",
                "params": ["${wake_at}", "${task_id}"],
                "queue_resume": match queue_primitive {
                    IiiQueuePrimitive::IiiQueuePreferred => "enqueue target function again when deadline matures",
                    IiiQueuePrimitive::DatabaseLeaseFallback => "database claim query sees visible_at when deadline matures",
                }
            }),
            ["sleep suspends work and schedules a future claim"],
        ));

        match queue_primitive {
            IiiQueuePrimitive::IiiQueuePreferred => calls.push(IiiFunctionCallPlan::runtime(
                IiiDurableOperation::Retry,
                json!({
                    "queue": self.queue_name,
                    "queue_config": {
                        "max_retries": "${max_attempts}",
                        "backoff_ms": "${base_backoff_ms}",
                        "type": "standard|fifo"
                    }
                }),
                ["iii-queue applies named queue retry/backoff and dead-letter behavior"],
            )),
            IiiQueuePrimitive::DatabaseLeaseFallback => calls.push(IiiFunctionCallPlan::function(
                IiiDurableOperation::Retry,
                "iii-database::execute",
                json!({
                    "db": "openview",
                    "sql": "UPDATE durable_tasks SET attempt = ?, visible_at = ?, lease_owner = NULL, lease_expires_at = NULL WHERE task_id = ?",
                    "params": ["${next_attempt}", "${retry_at}", "${task_id}"]
                }),
                ["task-level retry creates a new run that reuses completed checkpoints"],
            )),
        }

        match queue_primitive {
            IiiQueuePrimitive::IiiQueuePreferred => calls.push(IiiFunctionCallPlan::function(
                IiiDurableOperation::Cancel,
                "iii::durable::publish",
                json!({
                    "topic": "openview::cancel",
                    "data": {
                        "queue": self.queue_name,
                        "task": self.task_name,
                        "cancelled_at": "${now}"
                    }
                }),
                ["cancellation is published durably and observed at the next checkpoint or heartbeat"],
            )),
            IiiQueuePrimitive::DatabaseLeaseFallback => calls.push(IiiFunctionCallPlan::function(
                IiiDurableOperation::Cancel,
                "iii-database::execute",
                json!({
                    "db": "openview",
                    "sql": "UPDATE durable_tasks SET state = 'cancelled', cancelled_at = ? WHERE task_id = ?",
                    "params": ["${now}", "${task_id}"]
                }),
                ["cancellation is observed at the next checkpoint or heartbeat"],
            )),
        }

        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::ListPendingApprovals,
            "approval::list_pending",
            json!({ "session_id": self.session_id }),
            ["approval gate exposes pending human decisions"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::ResolveApproval,
            "approval::resolve",
            json!({
                "function_call_id": "${function_call_id}",
                "tool_call_id": "${tool_call_id}",
                "decision": "allow|deny",
                "reason": "${reason}"
            }),
            ["approval resolution records the human decision before resuming the run"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::EvidenceStreamAppend,
            "stream::set",
            json!({
                "stream_name": "openview::evidence",
                "group_id": self.session_id,
                "item_id": "${session_id}-${sequence}",
                "data": "${evidence_event}"
            }),
            ["evidence stream captures inspectable run artifacts and decisions"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::EvidenceStreamRead,
            "stream::list",
            json!({
                "stream_name": "openview::evidence",
                "group_id": self.session_id
            }),
            ["evidence stream can be replayed without reading secret payload bytes"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::HermesRunStart,
            "run::start",
            json!({
                "session_id": self.session_id,
                "provider": "openai",
                "model": "gpt-5.5",
                "reasoning_effort": "xhigh",
                "messages": [{
                    "role": "user",
                    "content": [{"type": "text", "text": self.run_goal}]
                }],
                "mode": "fire_and_forget"
            }),
            ["Hermes-compatible iii run starts asynchronously"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::HermesRunStartAndWait,
            "run::start_and_wait",
            json!({
                "session_id": self.session_id,
                "provider": "openai",
                "model": "gpt-5.5",
                "reasoning_effort": "xhigh",
                "messages": [{
                    "role": "user",
                    "content": [{"type": "text", "text": self.run_goal}]
                }],
                "mode": match self.hermes_mode {
                    IiiRunMode::Start => "available_for_tests",
                    IiiRunMode::StartAndWait => "selected",
                }
            }),
            ["Hermes-compatible iii run can block until terminal state for harness tests"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::SessionCreate,
            "session-tree::create",
            json!({ "display_name": format!("OpenView durable run {}", self.session_id) }),
            ["session tree creates durable conversation/run context"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::SessionAppend,
            "session-tree::append",
            json!({
                "session_id": self.session_id,
                "message": {
                    "role": "assistant|tool|system",
                    "content": "${message}",
                    "timestamp": "${unix_ms}"
                }
            }),
            ["session append records run messages and state transitions"],
        ));
        calls.push(IiiFunctionCallPlan::function(
            IiiDurableOperation::SessionMessages,
            "session-tree::messages",
            json!({ "session_id": self.session_id }),
            ["session messages reload durable run context after restart"],
        ));

        IiiBackendPlan {
            queue_primitive,
            planning_only: true,
            calls,
        }
    }
}

struct AgentRunnerManifestSpec<'a> {
    name: &'a str,
    display_name: &'a str,
    binary: &'a str,
    description: &'a str,
    env_keys: &'a [&'a str],
    argv_template: &'a [&'a str],
    profile_required: bool,
}

fn agent_runner_worker_manifest(spec: AgentRunnerManifestSpec<'_>) -> WorkerManifest {
    const WORKSPACE_ROOT: &str = "${OPENVIEW_WORKSPACE_ROOT}";
    const WORKTREE_ROOT: &str = "${OPENVIEW_WORKTREE_ROOT}";

    let required = if spec.profile_required {
        json!(["profile", "prompt", "workspace"])
    } else {
        json!(["prompt", "workspace"])
    };

    let mut sandbox = SandboxProfile::locked_down()
        .allow_workspace_read(WORKSPACE_ROOT)
        .allow_workspace_read(WORKTREE_ROOT)
        .allow_workspace_write(WORKTREE_ROOT)
        .allow_command(spec.binary)
        .allow_network_host("${OPENVIEW_AGENT_PROVIDER_HOST}");
    for env_key in spec.env_keys {
        sandbox = sandbox.allow_secret_name(*env_key);
    }

    let mut manifest = WorkerManifest::new(spec.name, "0.1.0")
        .description(spec.description)
        .target(BackendTarget::LocalBinary)
        .resource(ResourceKind::SessionState)
        .resource(ResourceKind::EventStream)
        .resource(ResourceKind::ApprovalQueue)
        .resource(ResourceKind::GitWorktree)
        .resource(ResourceKind::Filesystem)
        .resource(ResourceKind::Process)
        .resource(ResourceKind::CredentialVault)
        .resource(ResourceKind::ModelProvider)
        .dependency("git.worktree")
        .dependency("shell.sandbox")
        .dependency("approval.gate")
        .function(
            FunctionSpec::new(format!("{}::spawn_session", spec.name), CapabilityRisk::Exec)
                .description("Start or resume an agent CLI session in a scoped workspace/worktree")
                .request_schema(json!({
                    "type": "object",
                    "required": required,
                    "properties": {
                        "workspace": {"type": "string", "description": "Workspace or worktree directory under OpenView control"},
                        "prompt": {"type": "string", "description": "Initial task instruction"},
                        "model": {"type": ["string", "null"], "description": "Optional model override for this agent CLI"},
                        "profile": {"type": ["string", "null"], "description": "Optional CLI profile/config name"},
                        "session_id": {"type": ["string", "null"], "description": "Optional prior session id to resume"},
                        "worktree_id": {"type": ["string", "null"], "description": "OpenView worktree binding id"},
                        "skills": {"type": "array", "items": {"type": "string"}, "default": []},
                        "toolsets": {"type": "array", "items": {"type": "string"}, "default": []},
                        "approval_policy": {"type": "string", "enum": ["ask", "never", "yolo"], "default": "ask"},
                        "json_events": {"type": "boolean", "default": true},
                        "extra_args": {"type": "array", "items": {"type": "string"}, "default": []}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["agent", "session_id", "started", "event_stream"],
                    "properties": {
                        "agent": {"type": "string"},
                        "session_id": {"type": "string"},
                        "started": {"type": "boolean"},
                        "event_stream": {"type": "string"},
                        "job_id": {"type": ["string", "null"]},
                        "pid": {"type": ["integer", "null"]},
                        "worktree_id": {"type": ["string", "null"]},
                        "approval_queue": {"type": ["string", "null"]}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true)
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new(format!("{}::send_prompt", spec.name), CapabilityRisk::State)
                .description("Send a follow-up prompt to an existing agent CLI session")
                .request_schema(json!({
                    "type": "object",
                    "required": ["session_id", "prompt"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "prompt": {"type": "string"},
                        "approval_policy": {"type": "string", "enum": ["ask", "never", "yolo"], "default": "ask"}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["session_id", "accepted"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "accepted": {"type": "boolean"},
                        "event_stream": {"type": ["string", "null"]}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true)
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new(format!("{}::stream_events", spec.name), CapabilityRisk::Read)
                .description("Read normalized event output for an agent CLI session")
                .request_schema(json!({
                    "type": "object",
                    "required": ["session_id"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "after_sequence": {"type": "integer", "minimum": 0, "default": 0},
                        "limit": {"type": "integer", "minimum": 1, "default": 100}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["session_id", "events", "next_after_sequence"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "events": {"type": "array", "items": {"type": "object"}},
                        "next_after_sequence": {"type": "integer", "minimum": 0},
                        "has_more": {"type": "boolean"}
                    },
                    "additionalProperties": false
                }))
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new(format!("{}::resolve_approval", spec.name), CapabilityRisk::Approval)
                .description("Approve or deny a pending tool approval for an agent CLI session")
                .request_schema(json!({
                    "type": "object",
                    "required": ["session_id", "approval_id", "approved"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "approval_id": {"type": "string"},
                        "approved": {"type": "boolean"},
                        "reason": {"type": ["string", "null"]}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["session_id", "approval_id", "resolved"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "approval_id": {"type": "string"},
                        "resolved": {"type": "boolean"}
                    },
                    "additionalProperties": false
                }))
                .visibility(FunctionVisibility::Ui),
        )
        .sandbox(sandbox);

    manifest.display_name = spec.display_name.to_string();
    manifest.binary_name = Some(spec.binary.to_string());
    manifest.language = "local_cli".to_string();
    manifest.deploy_kind = "local_cli".to_string();
    manifest.default_config = json!({
        "binary": spec.binary,
        "argv_template": spec.argv_template,
        "env_keys": spec.env_keys,
        "event_stream": format!("openview.worker.{}.events", spec.name),
        "approval_queue": format!("openview.worker.{}.approvals", spec.name),
        "launch_strategy": "git.worktree + shell.sandbox exec_bg",
        "default_approval_policy": "ask"
    });
    manifest
}

pub fn codex_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "codex.agent",
        display_name: "Codex",
        binary: "codex",
        description: "Codex CLI runner for scoped non-interactive coding sessions in worktrees",
        env_keys: &["CODEX_HOME", "OPENAI_API_KEY"],
        argv_template: &[
            "exec",
            "--cd",
            "${workspace}",
            "--sandbox",
            "workspace-write",
            "--ask-for-approval",
            "on-request",
            "--json",
            "${prompt}",
        ],
        profile_required: false,
    })
}

pub fn claude_code_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "claude-code.agent",
        display_name: "Claude Code",
        binary: "claude",
        description: "Claude Code CLI runner for scoped coding sessions in worktrees",
        env_keys: &["ANTHROPIC_API_KEY", "CLAUDE_CONFIG_DIR"],
        argv_template: &["-p", "${prompt}"],
        profile_required: false,
    })
}

pub fn hermes_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "hermes.agent",
        display_name: "Hermes Agent",
        binary: "hermes",
        description: "Hermes Agent session lifecycle, prompt delivery, event streaming, and approval resolution",
        env_keys: &["HERMES_HOME", "HERMES_PROFILE"],
        argv_template: &[
            "--profile",
            "${profile}",
            "chat",
            "--query",
            "${prompt}",
            "--source",
            "openview-worker",
        ],
        profile_required: true,
    })
}

pub fn opencode_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "opencode.agent",
        display_name: "OpenCode",
        binary: "opencode",
        description: "OpenCode CLI runner for scoped coding sessions in worktrees",
        env_keys: &["OPENCODE_HOME", "OPENCODE_API_KEY"],
        argv_template: &["run", "${prompt}"],
        profile_required: false,
    })
}

pub fn openclaw_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "openclaw.agent",
        display_name: "OpenClaw",
        binary: "openclaw",
        description: "OpenClaw CLI runner for scoped agent sessions in worktrees",
        env_keys: &[
            "OPENCLAW_HOME",
            "OPENCLAW_PROFILE",
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
        ],
        argv_template: &[
            "--profile",
            "${profile}",
            "agent",
            "--json",
            "--message",
            "${prompt}",
        ],
        profile_required: true,
    })
}

pub fn gemini_cli_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "gemini-cli.agent",
        display_name: "Gemini CLI",
        binary: "gemini",
        description: "Gemini CLI runner for scoped non-interactive coding sessions in worktrees",
        env_keys: &["GEMINI_API_KEY", "GOOGLE_API_KEY", "GEMINI_CLI_HOME"],
        argv_template: &["-p", "${prompt}"],
        profile_required: false,
    })
}

pub fn goose_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "goose.agent",
        display_name: "Goose",
        binary: "goose",
        description: "Goose CLI runner for scoped coding sessions in worktrees",
        env_keys: &[
            "GOOSE_CONFIG_DIR",
            "GOOSE_PROVIDER",
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
        ],
        argv_template: &["run", "-t", "${prompt}"],
        profile_required: false,
    })
}

pub fn aider_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "aider.agent",
        display_name: "Aider",
        binary: "aider",
        description: "Aider CLI runner for scoped pair-programming sessions in worktrees",
        env_keys: &["AIDER_ENV_FILE", "OPENAI_API_KEY", "ANTHROPIC_API_KEY"],
        argv_template: &["--message", "${prompt}", "--yes-always"],
        profile_required: false,
    })
}

pub fn openhands_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "openhands.agent",
        display_name: "OpenHands",
        binary: "openhands",
        description: "OpenHands CLI runner for scoped headless coding sessions in worktrees",
        env_keys: &[
            "OPENHANDS_CONFIG_DIR",
            "LLM_API_KEY",
            "LLM_MODEL",
            "LLM_BASE_URL",
        ],
        argv_template: &["--headless", "-t", "${prompt}"],
        profile_required: false,
    })
}

pub fn crush_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "crush.agent",
        display_name: "Crush",
        binary: "crush",
        description: "Crush CLI runner for scoped non-interactive coding sessions in worktrees",
        env_keys: &[
            "CRUSH_CONFIG_DIR",
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "GEMINI_API_KEY",
        ],
        argv_template: &["run", "${prompt}"],
        profile_required: false,
    })
}

pub fn qwen_code_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "qwen-code.agent",
        display_name: "Qwen Code",
        binary: "qwen",
        description: "Qwen Code CLI runner for scoped non-interactive coding sessions in worktrees",
        env_keys: &["DASHSCOPE_API_KEY", "OPENAI_API_KEY", "QWEN_CODE_HOME"],
        argv_template: &["-p", "${prompt}"],
        profile_required: false,
    })
}

pub fn cursor_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "cursor-agent.agent",
        display_name: "Cursor Agent",
        binary: "cursor-agent",
        description:
            "Cursor Agent CLI runner for scoped non-interactive coding sessions in worktrees",
        env_keys: &["CURSOR_API_KEY", "CURSOR_CONFIG_DIR"],
        argv_template: &["-p", "${prompt}", "--output-format", "json"],
        profile_required: false,
    })
}

pub fn amp_agent_worker_manifest() -> WorkerManifest {
    agent_runner_worker_manifest(AgentRunnerManifestSpec {
        name: "amp.agent",
        display_name: "Amp",
        binary: "amp",
        description: "Amp CLI runner for scoped execute-mode coding sessions in worktrees",
        env_keys: &["AMP_API_KEY", "AMP_SETTINGS_FILE"],
        argv_template: &["--execute", "${prompt}", "--stream-json"],
        profile_required: false,
    })
}

pub fn agent_runner_worker_manifests() -> Vec<WorkerManifest> {
    vec![
        codex_agent_worker_manifest(),
        claude_code_agent_worker_manifest(),
        hermes_agent_worker_manifest(),
        opencode_agent_worker_manifest(),
        openclaw_agent_worker_manifest(),
        gemini_cli_agent_worker_manifest(),
        goose_agent_worker_manifest(),
        aider_agent_worker_manifest(),
        openhands_agent_worker_manifest(),
        crush_agent_worker_manifest(),
        qwen_code_agent_worker_manifest(),
        cursor_agent_worker_manifest(),
        amp_agent_worker_manifest(),
    ]
}

pub fn built_in_worker_catalog() -> Vec<WorkerManifest> {
    let mut workers = vec![
        git_worktree_worker_manifest(),
        terminal_pty_worker_manifest(),
    ];
    workers.extend(agent_runner_worker_manifests());
    workers.extend([
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
    ]);
    workers
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
