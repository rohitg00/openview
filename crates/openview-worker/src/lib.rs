pub use openview_core::{git_worktree_worker_manifest, terminal_pty_worker_manifest};
use openview_core::{
    BackendTarget, CapabilityRisk, FunctionSpec, FunctionVisibility, ResourceKind, SandboxProfile,
    WorkerManifest,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkerManifestError {
    #[error("manifest JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BinaryWorkerCli {
    pub url: String,
    pub config_path: Option<String>,
    pub manifest_only: bool,
}

impl Default for BinaryWorkerCli {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:49134".to_string(),
            config_path: None,
            manifest_only: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HermesApprovalPolicy {
    #[default]
    Ask,
    Yolo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HermesSessionRequest {
    pub workspace: String,
    pub profile: String,
    pub prompt: String,
    pub session_id: Option<String>,
    pub skills: Vec<String>,
    pub toolsets: Vec<String>,
    pub approval_policy: HermesApprovalPolicy,
}

impl HermesSessionRequest {
    pub fn new(
        workspace: impl Into<String>,
        profile: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            profile: profile.into(),
            prompt: prompt.into(),
            session_id: None,
            skills: Vec::new(),
            toolsets: Vec::new(),
            approval_policy: HermesApprovalPolicy::default(),
        }
    }

    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn skill(mut self, skill: impl Into<String>) -> Self {
        self.skills.push(skill.into());
        self
    }

    pub fn skills(mut self, skills: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.skills.extend(skills.into_iter().map(Into::into));
        self
    }

    pub fn toolset(mut self, toolset: impl Into<String>) -> Self {
        self.toolsets.push(toolset.into());
        self
    }

    pub fn toolsets(mut self, toolsets: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.toolsets.extend(toolsets.into_iter().map(Into::into));
        self
    }

    pub fn approval_policy(mut self, approval_policy: HermesApprovalPolicy) -> Self {
        self.approval_policy = approval_policy;
        self
    }

    pub fn command_invocation(&self) -> HermesCommandInvocation {
        let mut argv = vec!["--profile".to_string(), self.profile.clone()];

        if let Some(session_id) = &self.session_id {
            argv.push("--resume".to_string());
            argv.push(session_id.clone());
        }

        if !self.skills.is_empty() {
            argv.push("--skills".to_string());
            argv.push(self.skills.join(","));
        }

        if self.approval_policy == HermesApprovalPolicy::Yolo {
            argv.push("--yolo".to_string());
        }

        argv.push("chat".to_string());
        argv.push("--query".to_string());
        argv.push(self.prompt.clone());

        if !self.toolsets.is_empty() {
            argv.push("--toolsets".to_string());
            argv.push(self.toolsets.join(","));
        }

        argv.push("--source".to_string());
        argv.push("openview-worker".to_string());

        HermesCommandInvocation {
            binary: "hermes".to_string(),
            argv,
            env_keys: vec!["HERMES_HOME".to_string(), "HERMES_PROFILE".to_string()],
            working_directory: Some(self.workspace.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HermesCommandInvocation {
    pub binary: String,
    pub argv: Vec<String>,
    pub env_keys: Vec<String>,
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkerProcessRequest {
    pub worker_id: String,
    pub binary: String,
    pub argv: Vec<String>,
    pub env_keys: Vec<String>,
    pub working_directory: Option<String>,
    pub create_process_group: bool,
    pub readiness_probe: Option<ReadinessProbe>,
    pub restart_policy: RestartPolicy,
    pub stop_policy: StopPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_lease_heartbeat: Option<WorkerProcessQueueLeaseHeartbeatPolicy>,
}

impl WorkerProcessRequest {
    pub fn new(worker_id: impl Into<String>, binary: impl Into<String>) -> Self {
        Self {
            worker_id: worker_id.into(),
            binary: binary.into(),
            argv: Vec::new(),
            env_keys: Vec::new(),
            working_directory: None,
            create_process_group: false,
            readiness_probe: None,
            restart_policy: RestartPolicy::Never,
            stop_policy: StopPolicy::term_then_kill(5_000),
            queue_lease_heartbeat: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.argv.push(arg.into());
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.argv.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env_key(mut self, key: impl Into<String>) -> Self {
        self.env_keys.push(key.into());
        self
    }

    pub fn env_keys(mut self, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.env_keys.extend(keys.into_iter().map(Into::into));
        self
    }

    pub fn working_directory(mut self, working_directory: impl Into<String>) -> Self {
        self.working_directory = Some(working_directory.into());
        self
    }

    pub fn process_group(mut self, create_process_group: bool) -> Self {
        self.create_process_group = create_process_group;
        self
    }

    pub fn readiness_probe(mut self, readiness_probe: ReadinessProbe) -> Self {
        self.readiness_probe = Some(readiness_probe);
        self
    }

    pub fn restart_policy(mut self, restart_policy: RestartPolicy) -> Self {
        self.restart_policy = restart_policy;
        self
    }

    pub fn stop_policy(mut self, stop_policy: StopPolicy) -> Self {
        self.stop_policy = stop_policy;
        self
    }

    pub fn queue_lease_heartbeat(mut self, policy: WorkerProcessQueueLeaseHeartbeatPolicy) -> Self {
        self.queue_lease_heartbeat = Some(policy);
        self
    }

    pub fn invocation(&self) -> WorkerProcessInvocation {
        WorkerProcessInvocation {
            worker_id: self.worker_id.clone(),
            binary: self.binary.clone(),
            argv: self.argv.clone(),
            env_keys: self.env_keys.clone(),
            working_directory: self.working_directory.clone(),
            create_process_group: self.create_process_group,
            readiness_probe: self.readiness_probe.clone(),
            restart_policy: self.restart_policy.clone(),
            stop_policy: self.stop_policy.clone(),
            queue_lease_heartbeat: self.queue_lease_heartbeat.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkerProcessInvocation {
    pub worker_id: String,
    pub binary: String,
    pub argv: Vec<String>,
    pub env_keys: Vec<String>,
    pub working_directory: Option<String>,
    pub create_process_group: bool,
    pub readiness_probe: Option<ReadinessProbe>,
    pub restart_policy: RestartPolicy,
    pub stop_policy: StopPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_lease_heartbeat: Option<WorkerProcessQueueLeaseHeartbeatPolicy>,
}

pub const WORKER_PROCESS_QUEUE_LEASE_HEARTBEAT_CONTROL_API_METHOD: &str =
    "OpenViewControlApi::heartbeat_worker_leases";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkerProcessQueueLeaseHeartbeatPolicy {
    pub queue: String,
    pub visibility_timeout_ms: i64,
    #[serde(default)]
    pub metadata_keys: Vec<String>,
}

impl WorkerProcessQueueLeaseHeartbeatPolicy {
    pub fn new(queue: impl Into<String>, visibility_timeout_ms: i64) -> Self {
        Self {
            queue: queue.into(),
            visibility_timeout_ms,
            metadata_keys: Vec::new(),
        }
    }

    pub fn metadata_key(mut self, key: impl Into<String>) -> Self {
        self.metadata_keys.push(key.into());
        self
    }

    pub fn metadata_keys(mut self, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.metadata_keys.extend(keys.into_iter().map(Into::into));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkerProcessState {
    Idle,
    StartRequested,
    Ready,
    StopRequested,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WorkerProcessSupervisorEvent {
    StartRequested {
        sequence: u64,
        worker_id: String,
        attempt: u32,
        invocation: WorkerProcessInvocation,
    },
    Ready {
        sequence: u64,
        worker_id: String,
        attempt: u32,
        readiness_probe: Option<ReadinessProbe>,
        message: String,
    },
    Log {
        sequence: u64,
        worker_id: String,
        attempt: u32,
        stream: WorkerStdioStream,
        payload: serde_json::Value,
    },
    Heartbeat {
        sequence: u64,
        worker_id: String,
        attempt: u32,
        queue: String,
        visibility_timeout_ms: i64,
        metadata: serde_json::Value,
        control_api_method: String,
    },
    ExitFailure {
        sequence: u64,
        worker_id: String,
        attempt: u32,
        exit_code: i32,
        message: String,
    },
    RestartScheduled {
        sequence: u64,
        worker_id: String,
        next_attempt: u32,
        restart_count: u32,
        backoff_ms: u64,
    },
    GracefulStopRequested {
        sequence: u64,
        worker_id: String,
        attempt: u32,
        stop_policy: StopPolicy,
        reason: String,
    },
    Stopped {
        sequence: u64,
        worker_id: String,
        attempt: u32,
        exit_code: i32,
        message: String,
    },
    Failed {
        sequence: u64,
        worker_id: String,
        attempt: u32,
        reason: String,
    },
}

impl WorkerProcessSupervisorEvent {
    pub fn sequence(&self) -> u64 {
        match self {
            Self::StartRequested { sequence, .. }
            | Self::Ready { sequence, .. }
            | Self::Log { sequence, .. }
            | Self::Heartbeat { sequence, .. }
            | Self::ExitFailure { sequence, .. }
            | Self::RestartScheduled { sequence, .. }
            | Self::GracefulStopRequested { sequence, .. }
            | Self::Stopped { sequence, .. }
            | Self::Failed { sequence, .. } => *sequence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkerProcessSupervisor {
    invocation: WorkerProcessInvocation,
    state: WorkerProcessState,
    attempt: u32,
    restart_count: u32,
    next_sequence: u64,
    redaction_rules: Vec<RedactionRule>,
    events: Vec<WorkerProcessSupervisorEvent>,
}

impl WorkerProcessSupervisor {
    pub fn new(invocation: WorkerProcessInvocation) -> Self {
        Self {
            invocation,
            state: WorkerProcessState::Idle,
            attempt: 1,
            restart_count: 0,
            next_sequence: 1,
            redaction_rules: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn with_redaction_rules(mut self, redaction_rules: Vec<RedactionRule>) -> Self {
        self.redaction_rules = redaction_rules;
        self
    }

    pub fn state(&self) -> WorkerProcessState {
        self.state
    }

    pub fn restart_count(&self) -> u32 {
        self.restart_count
    }

    pub fn events(&self) -> &[WorkerProcessSupervisorEvent] {
        &self.events
    }

    pub fn request_start(&mut self) -> WorkerProcessSupervisorEvent {
        self.push_start_requested()
    }

    pub fn mark_ready(&mut self, message: impl Into<String>) -> WorkerProcessSupervisorEvent {
        self.state = WorkerProcessState::Ready;
        self.push_event(WorkerProcessSupervisorEvent::Ready {
            sequence: 0,
            worker_id: self.invocation.worker_id.clone(),
            attempt: self.attempt,
            readiness_probe: self.invocation.readiness_probe.clone(),
            message: message.into(),
        })
    }

    pub fn record_stdout(&mut self, text: impl AsRef<str>) -> WorkerProcessSupervisorEvent {
        self.record_log(WorkerStdioStream::Stdout, text)
    }

    pub fn record_stderr(&mut self, text: impl AsRef<str>) -> WorkerProcessSupervisorEvent {
        self.record_log(WorkerStdioStream::Stderr, text)
    }

    pub fn record_queue_lease_heartbeat(
        &mut self,
        request: &openview_core::HeartbeatWorkerLeaseRequest,
    ) -> Option<WorkerProcessSupervisorEvent> {
        if self.state != WorkerProcessState::Ready || request.worker_id != self.invocation.worker_id
        {
            return None;
        }

        let policy = self.invocation.queue_lease_heartbeat.as_ref()?;
        let visibility_timeout_ms = request.visibility_timeout.num_milliseconds();
        if request.queue != policy.queue || visibility_timeout_ms != policy.visibility_timeout_ms {
            return None;
        }

        let metadata = sanitized_heartbeat_metadata(
            request.metadata.as_ref(),
            &policy.metadata_keys,
            &self.redaction_rules,
        );

        Some(self.push_event(WorkerProcessSupervisorEvent::Heartbeat {
            sequence: 0,
            worker_id: self.invocation.worker_id.clone(),
            attempt: self.attempt,
            queue: policy.queue.clone(),
            visibility_timeout_ms,
            metadata,
            control_api_method: WORKER_PROCESS_QUEUE_LEASE_HEARTBEAT_CONTROL_API_METHOD.to_string(),
        }))
    }

    pub fn record_exit_failure(
        &mut self,
        exit_code: i32,
        message: impl Into<String>,
    ) -> WorkerProcessSupervisorEvent {
        let failure = self.push_event(WorkerProcessSupervisorEvent::ExitFailure {
            sequence: 0,
            worker_id: self.invocation.worker_id.clone(),
            attempt: self.attempt,
            exit_code,
            message: message.into(),
        });

        match self.invocation.restart_policy {
            RestartPolicy::Never => {
                self.state = WorkerProcessState::Failed;
                self.push_failed("process exited with no restart policy".to_string());
            }
            RestartPolicy::OnFailure {
                max_restarts,
                backoff_ms,
            } => {
                if self.restart_count < max_restarts {
                    self.restart_count += 1;
                    let next_attempt = self.attempt + 1;
                    self.push_event(WorkerProcessSupervisorEvent::RestartScheduled {
                        sequence: 0,
                        worker_id: self.invocation.worker_id.clone(),
                        next_attempt,
                        restart_count: self.restart_count,
                        backoff_ms,
                    });
                    self.attempt = next_attempt;
                    self.push_start_requested();
                } else {
                    self.state = WorkerProcessState::Failed;
                    self.push_failed(format!(
                        "max restarts exceeded after {} restarts",
                        self.restart_count
                    ));
                }
            }
        }

        failure
    }

    pub fn request_graceful_stop(
        &mut self,
        reason: impl Into<String>,
    ) -> WorkerProcessSupervisorEvent {
        self.state = WorkerProcessState::StopRequested;
        self.push_event(WorkerProcessSupervisorEvent::GracefulStopRequested {
            sequence: 0,
            worker_id: self.invocation.worker_id.clone(),
            attempt: self.attempt,
            stop_policy: self.invocation.stop_policy.clone(),
            reason: reason.into(),
        })
    }

    pub fn mark_stopped(
        &mut self,
        exit_code: i32,
        message: impl Into<String>,
    ) -> WorkerProcessSupervisorEvent {
        self.state = WorkerProcessState::Stopped;
        self.push_event(WorkerProcessSupervisorEvent::Stopped {
            sequence: 0,
            worker_id: self.invocation.worker_id.clone(),
            attempt: self.attempt,
            exit_code,
            message: message.into(),
        })
    }

    fn record_log(
        &mut self,
        stream: WorkerStdioStream,
        text: impl AsRef<str>,
    ) -> WorkerProcessSupervisorEvent {
        let sequence = self.next_sequence;
        let payload = WorkerProcessStdio::event_payload(
            self.invocation.worker_id.clone(),
            stream.clone(),
            sequence,
            text,
            &self.redaction_rules,
        );
        self.push_event(WorkerProcessSupervisorEvent::Log {
            sequence: 0,
            worker_id: self.invocation.worker_id.clone(),
            attempt: self.attempt,
            stream,
            payload,
        })
    }

    fn push_start_requested(&mut self) -> WorkerProcessSupervisorEvent {
        self.state = WorkerProcessState::StartRequested;
        self.push_event(WorkerProcessSupervisorEvent::StartRequested {
            sequence: 0,
            worker_id: self.invocation.worker_id.clone(),
            attempt: self.attempt,
            invocation: self.invocation.clone(),
        })
    }

    fn push_failed(&mut self, reason: String) -> WorkerProcessSupervisorEvent {
        self.push_event(WorkerProcessSupervisorEvent::Failed {
            sequence: 0,
            worker_id: self.invocation.worker_id.clone(),
            attempt: self.attempt,
            reason,
        })
    }

    fn push_event(&mut self, event: WorkerProcessSupervisorEvent) -> WorkerProcessSupervisorEvent {
        let event = with_sequence(event, self.next_sequence);
        self.next_sequence += 1;
        self.events.push(event.clone());
        event
    }
}

fn with_sequence(
    event: WorkerProcessSupervisorEvent,
    sequence: u64,
) -> WorkerProcessSupervisorEvent {
    match event {
        WorkerProcessSupervisorEvent::StartRequested {
            worker_id,
            attempt,
            invocation,
            ..
        } => WorkerProcessSupervisorEvent::StartRequested {
            sequence,
            worker_id,
            attempt,
            invocation,
        },
        WorkerProcessSupervisorEvent::Ready {
            worker_id,
            attempt,
            readiness_probe,
            message,
            ..
        } => WorkerProcessSupervisorEvent::Ready {
            sequence,
            worker_id,
            attempt,
            readiness_probe,
            message,
        },
        WorkerProcessSupervisorEvent::Log {
            worker_id,
            attempt,
            stream,
            payload,
            ..
        } => WorkerProcessSupervisorEvent::Log {
            sequence,
            worker_id,
            attempt,
            stream,
            payload,
        },
        WorkerProcessSupervisorEvent::Heartbeat {
            worker_id,
            attempt,
            queue,
            visibility_timeout_ms,
            metadata,
            control_api_method,
            ..
        } => WorkerProcessSupervisorEvent::Heartbeat {
            sequence,
            worker_id,
            attempt,
            queue,
            visibility_timeout_ms,
            metadata,
            control_api_method,
        },
        WorkerProcessSupervisorEvent::ExitFailure {
            worker_id,
            attempt,
            exit_code,
            message,
            ..
        } => WorkerProcessSupervisorEvent::ExitFailure {
            sequence,
            worker_id,
            attempt,
            exit_code,
            message,
        },
        WorkerProcessSupervisorEvent::RestartScheduled {
            worker_id,
            next_attempt,
            restart_count,
            backoff_ms,
            ..
        } => WorkerProcessSupervisorEvent::RestartScheduled {
            sequence,
            worker_id,
            next_attempt,
            restart_count,
            backoff_ms,
        },
        WorkerProcessSupervisorEvent::GracefulStopRequested {
            worker_id,
            attempt,
            stop_policy,
            reason,
            ..
        } => WorkerProcessSupervisorEvent::GracefulStopRequested {
            sequence,
            worker_id,
            attempt,
            stop_policy,
            reason,
        },
        WorkerProcessSupervisorEvent::Stopped {
            worker_id,
            attempt,
            exit_code,
            message,
            ..
        } => WorkerProcessSupervisorEvent::Stopped {
            sequence,
            worker_id,
            attempt,
            exit_code,
            message,
        },
        WorkerProcessSupervisorEvent::Failed {
            worker_id,
            attempt,
            reason,
            ..
        } => WorkerProcessSupervisorEvent::Failed {
            sequence,
            worker_id,
            attempt,
            reason,
        },
    }
}

fn sanitized_heartbeat_metadata(
    metadata: Option<&serde_json::Value>,
    allowed_keys: &[String],
    rules: &[RedactionRule],
) -> serde_json::Value {
    let mut sanitized = serde_json::Map::new();
    let Some(serde_json::Value::Object(source)) = metadata else {
        return serde_json::Value::Object(sanitized);
    };

    for key in allowed_keys {
        if is_sensitive_metadata_key(key) {
            continue;
        }

        if let Some(value) = source.get(key) {
            sanitized.insert(key.clone(), sanitize_heartbeat_metadata_value(value, rules));
        }
    }

    serde_json::Value::Object(sanitized)
}

fn sanitize_heartbeat_metadata_value(
    value: &serde_json::Value,
    rules: &[RedactionRule],
) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .iter()
                .map(|value| sanitize_heartbeat_metadata_value(value, rules))
                .collect(),
        ),
        serde_json::Value::Object(entries) => {
            let mut sanitized = serde_json::Map::new();
            for (key, value) in entries {
                if is_sensitive_metadata_key(key) {
                    continue;
                }
                sanitized.insert(key.clone(), sanitize_heartbeat_metadata_value(value, rules));
            }
            serde_json::Value::Object(sanitized)
        }
        serde_json::Value::String(text) => {
            let mut sanitized = text.clone();
            for rule in rules {
                sanitized = rule.apply(&sanitized).0;
            }
            serde_json::Value::String(sanitized)
        }
        other => other.clone(),
    }
}

fn is_sensitive_metadata_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("secret")
        || key.contains("token")
        || key.contains("password")
        || key.contains("passwd")
        || key.contains("credential")
        || key.contains("api_key")
        || key.contains("apikey")
        || key == "key"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ReadinessProbe {
    Tcp {
        host: String,
        port: u16,
        initial_delay_ms: u64,
    },
}

impl ReadinessProbe {
    pub fn tcp(host: impl Into<String>, port: u16) -> Self {
        Self::Tcp {
            host: host.into(),
            port,
            initial_delay_ms: 0,
        }
    }

    pub fn initial_delay_ms(mut self, delay_ms: u64) -> Self {
        match &mut self {
            Self::Tcp {
                initial_delay_ms, ..
            } => *initial_delay_ms = delay_ms,
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RestartPolicy {
    Never,
    OnFailure { max_restarts: u32, backoff_ms: u64 },
}

impl RestartPolicy {
    pub fn on_failure(max_restarts: u32) -> Self {
        Self::OnFailure {
            max_restarts,
            backoff_ms: 0,
        }
    }

    pub fn backoff_ms(mut self, backoff_ms: u64) -> Self {
        if let Self::OnFailure {
            backoff_ms: slot, ..
        } = &mut self
        {
            *slot = backoff_ms;
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StopPolicy {
    pub term_timeout_ms: u64,
    pub kill_after_timeout: bool,
}

impl StopPolicy {
    pub fn term_then_kill(term_timeout_ms: u64) -> Self {
        Self {
            term_timeout_ms,
            kill_after_timeout: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RedactionRule {
    Literal {
        pattern: String,
        replacement: String,
    },
    EnvAssignment {
        key: String,
        replacement: String,
    },
}

impl RedactionRule {
    pub fn literal(pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self::Literal {
            pattern: pattern.into(),
            replacement: replacement.into(),
        }
    }

    pub fn env_assignment(key: impl Into<String>) -> Self {
        Self::EnvAssignment {
            key: key.into(),
            replacement: "[redacted]".to_string(),
        }
    }

    fn apply(&self, input: &str) -> (String, bool) {
        match self {
            Self::Literal {
                pattern,
                replacement,
            } => {
                if pattern.is_empty() || !input.contains(pattern) {
                    (input.to_string(), false)
                } else {
                    (input.replace(pattern, replacement), true)
                }
            }
            Self::EnvAssignment { key, replacement } => {
                let prefix = format!("{key}=");
                let Some(start) = input.find(&prefix) else {
                    return (input.to_string(), false);
                };
                let value_start = start + prefix.len();
                let value_len = input[value_start..]
                    .find(char::is_whitespace)
                    .unwrap_or(input.len() - value_start);
                let value_end = value_start + value_len;
                let mut sanitized = String::with_capacity(input.len() + replacement.len());
                sanitized.push_str(&input[..value_start]);
                sanitized.push_str(replacement);
                sanitized.push_str(&input[value_end..]);
                (sanitized, true)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStdioStream {
    Stdout,
    Stderr,
}

impl WorkerStdioStream {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }
}

pub struct WorkerProcessStdio;

impl WorkerProcessStdio {
    pub fn event_payload(
        worker_id: impl Into<String>,
        stream: WorkerStdioStream,
        sequence: u64,
        text: impl AsRef<str>,
        rules: &[RedactionRule],
    ) -> serde_json::Value {
        let mut sanitized = text.as_ref().to_string();
        let mut redacted = false;

        for rule in rules {
            let (next, changed) = rule.apply(&sanitized);
            sanitized = next;
            redacted |= changed;
        }

        json!({
            "worker_id": worker_id.into(),
            "stream": stream.as_str(),
            "sequence": sequence,
            "text": sanitized,
            "redacted": redacted,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CredentialReference {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl CredentialReference {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: None,
        }
    }

    pub fn with_value(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: Some(value.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkerExecutionDecisionKind {
    Allowed,
    Blocked,
    ApprovalRequired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkerExecutionDecision {
    pub kind: WorkerExecutionDecisionKind,
    pub reasons: Vec<String>,
    pub payload_summary: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkerExecutionRequest {
    pub worker_id: String,
    pub function_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub argv: Vec<String>,
    #[serde(default)]
    pub read_paths: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub network_hosts: Vec<String>,
    #[serde(default)]
    pub credential_refs: Vec<CredentialReference>,
    #[serde(default)]
    pub payload: serde_json::Value,
}

impl WorkerExecutionRequest {
    pub fn new(worker_id: impl Into<String>, function_id: impl Into<String>) -> Self {
        Self {
            worker_id: worker_id.into(),
            function_id: function_id.into(),
            command: None,
            argv: Vec::new(),
            read_paths: Vec::new(),
            write_paths: Vec::new(),
            network_hosts: Vec::new(),
            credential_refs: Vec::new(),
            payload: json!({}),
        }
    }

    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.argv.push(arg.into());
        self
    }

    pub fn read_path(mut self, path: impl Into<String>) -> Self {
        self.read_paths.push(path.into());
        self
    }

    pub fn write_path(mut self, path: impl Into<String>) -> Self {
        self.write_paths.push(path.into());
        self
    }

    pub fn network_host(mut self, host: impl Into<String>) -> Self {
        self.network_hosts.push(host.into());
        self
    }

    pub fn credential(mut self, credential: CredentialReference) -> Self {
        self.credential_refs.push(credential);
        self
    }

    pub fn payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn evaluate_against(
        &self,
        manifest: &WorkerManifest,
        resolved_approval_function_ids: &[&str],
    ) -> WorkerExecutionDecision {
        let mut reasons = Vec::new();

        if manifest.name != self.worker_id {
            reasons.push(format!("worker mismatch: {}", self.worker_id));
        }

        let sandbox = manifest.sandbox.as_ref();
        if sandbox.is_none() {
            reasons.push("sandbox policy missing".to_string());
        }

        if let (Some(sandbox), Some(command)) = (sandbox, &self.command) {
            if !sandbox.can_execute_command(command) {
                reasons.push(format!("command denied: {command}"));
            }
        }

        if let Some(sandbox) = sandbox {
            for path in &self.read_paths {
                if !sandbox.can_read_path(path) {
                    reasons.push(format!("read path denied: {path}"));
                }
            }

            for path in &self.write_paths {
                if !sandbox.can_write_path(path) {
                    reasons.push(format!("write path denied: {path}"));
                }
            }

            for host in &self.network_hosts {
                if !sandbox.can_access_network_host(host) {
                    reasons.push(format!("network host denied: {host}"));
                }
            }

            for credential in &self.credential_refs {
                if !sandbox.can_reference_secret_name(&credential.name) {
                    reasons.push(format!("credential reference denied: {}", credential.name));
                }
            }
        }

        let payload_summary = self.payload_summary();
        if !reasons.is_empty() {
            return WorkerExecutionDecision {
                kind: WorkerExecutionDecisionKind::Blocked,
                reasons,
                payload_summary,
            };
        }

        if manifest
            .security
            .approval_required_functions
            .contains(&self.function_id)
            && !resolved_approval_function_ids
                .iter()
                .any(|function_id| *function_id == self.function_id)
        {
            return WorkerExecutionDecision {
                kind: WorkerExecutionDecisionKind::ApprovalRequired,
                reasons: vec![format!("approval required: {}", self.function_id)],
                payload_summary,
            };
        }

        WorkerExecutionDecision {
            kind: WorkerExecutionDecisionKind::Allowed,
            reasons,
            payload_summary,
        }
    }

    fn payload_summary(&self) -> serde_json::Value {
        let secret_values = self
            .credential_refs
            .iter()
            .filter_map(|credential| credential.value.as_deref())
            .collect::<Vec<_>>();
        json!({
            "worker_id": self.worker_id,
            "function_id": self.function_id,
            "command": self.command,
            "argv": redact_value(json!(self.argv), &secret_values),
            "read_paths": self.read_paths,
            "write_paths": self.write_paths,
            "network_hosts": self.network_hosts,
            "credential_refs": self.credential_refs.iter().map(|credential| credential.name.clone()).collect::<Vec<_>>(),
            "payload": redact_value(self.payload.clone(), &secret_values),
        })
    }
}

fn redact_value(value: serde_json::Value, secret_values: &[&str]) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(|value| redact_value(value, secret_values))
                .collect(),
        ),
        serde_json::Value::Object(entries) => serde_json::Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| {
                    if is_sensitive_payload_key(&key) {
                        (key, json!("[redacted]"))
                    } else {
                        (key, redact_value(value, secret_values))
                    }
                })
                .collect(),
        ),
        serde_json::Value::String(text) => {
            if secret_values
                .iter()
                .any(|secret| !secret.is_empty() && *secret == text)
            {
                json!("[redacted]")
            } else {
                serde_json::Value::String(text)
            }
        }
        value => value,
    }
}

fn is_sensitive_payload_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "secret",
        "token",
        "password",
        "api_key",
        "credential",
        "private_key",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct IiiTriggerInvocation {
    pub function_id: String,
    pub payload: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl IiiTriggerInvocation {
    pub fn new(function_id: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            function_id: function_id.into(),
            payload,
            action: None,
            timeout_ms: None,
        }
    }

    pub fn action(mut self, action: serde_json::Value) -> Self {
        self.action = Some(action);
        self
    }

    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IiiTriggerPlan {
    pub purpose: String,
    pub calls: Vec<IiiTriggerInvocation>,
    pub required_workers: Vec<String>,
    pub resources: Vec<ResourceKind>,
    pub risks: Vec<CapabilityRisk>,
}

impl IiiTriggerPlan {
    pub fn new(purpose: impl Into<String>, calls: Vec<IiiTriggerInvocation>) -> Self {
        Self {
            purpose: purpose.into(),
            calls,
            required_workers: Vec::new(),
            resources: Vec::new(),
            risks: Vec::new(),
        }
    }

    pub fn worker(mut self, worker: impl Into<String>) -> Self {
        self.required_workers.push(worker.into());
        self
    }

    pub fn resource(mut self, resource: ResourceKind) -> Self {
        self.resources.push(resource);
        self
    }

    pub fn risk(mut self, risk: CapabilityRisk) -> Self {
        self.risks.push(risk);
        self
    }
}

fn openview_queue_database_plan(
    purpose: impl Into<String>,
    calls: Vec<IiiTriggerInvocation>,
) -> IiiTriggerPlan {
    IiiTriggerPlan::new(purpose, calls)
        .worker("iii-database")
        .resource(ResourceKind::Database)
        .risk(CapabilityRisk::Write)
        .risk(CapabilityRisk::State)
}

fn openview_queue_database_publish_plan(
    purpose: impl Into<String>,
    calls: Vec<IiiTriggerInvocation>,
) -> IiiTriggerPlan {
    openview_queue_database_plan(purpose, calls)
        .worker("iii-queue")
        .resource(ResourceKind::EventStream)
        .risk(CapabilityRisk::Stream)
}

pub const OPENVIEW_QUEUE_SCHEMA_VERSION: &str = "openview.queue.v1";
pub const OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID: &str =
    "openview.queue.v1.bootstrap_durable_queue_tables";
pub const OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION: &str = "openview.queue::bootstrap_schema";
pub const OPENVIEW_QUEUE_TABLE_NAMES: &[&str] = &[
    "durable_tasks",
    "durable_task_events",
    "durable_dead_letters",
    "durable_idempotency_keys",
];
pub const OPENVIEW_QUEUE_INDEX_NAMES: &[&str] = &[
    "idx_durable_tasks_claim_ready",
    "idx_durable_tasks_recover_leases",
    "idx_durable_tasks_recover_retries",
    "idx_durable_tasks_lease_owner",
    "idx_durable_task_events_task_created",
    "idx_durable_dead_letters_created",
    "idx_durable_idempotency_task_id",
];

pub fn openview_queue_schema_bootstrap_plan(db: impl Into<String>) -> IiiTriggerPlan {
    let db = db.into();
    let transaction = iii_database_transaction_invocation(
        db,
        vec![
            json!({
                "sql": "CREATE TABLE IF NOT EXISTS durable_tasks (task_id TEXT PRIMARY KEY, queue TEXT NOT NULL, task_name TEXT NOT NULL, payload_ref TEXT NOT NULL, payload_sha256 TEXT NOT NULL, state TEXT NOT NULL CHECK (state IN ('queued', 'leased', 'retrying', 'completed', 'dead_lettered', 'cancelled')), attempt INTEGER NOT NULL DEFAULT 0, visible_at TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, lease_id TEXT, lease_owner TEXT, leased_at TEXT, lease_expires_at TEXT, last_heartbeat_at TEXT, output_ref TEXT, output_sha256 TEXT, completed_at TEXT, failure_ref TEXT, failure_kind TEXT, dead_lettered_at TEXT, recovered_by TEXT, recovered_at TEXT)",
                "params": []
            }),
            json!({
                "sql": "CREATE TABLE IF NOT EXISTS durable_task_events (event_id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY (task_id) REFERENCES durable_tasks(task_id) ON DELETE CASCADE)",
                "params": []
            }),
            json!({
                "sql": "CREATE TABLE IF NOT EXISTS durable_dead_letters (task_id TEXT PRIMARY KEY, failure_ref TEXT NOT NULL, failure_kind TEXT NOT NULL, worker_id TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY (task_id) REFERENCES durable_tasks(task_id) ON DELETE CASCADE)",
                "params": []
            }),
            json!({
                "sql": "CREATE TABLE IF NOT EXISTS durable_idempotency_keys (idempotency_key TEXT PRIMARY KEY, task_id TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
                "params": []
            }),
            json!({
                "sql": "CREATE INDEX IF NOT EXISTS idx_durable_tasks_claim_ready ON durable_tasks (queue, state, visible_at, created_at) WHERE state IN ('queued', 'retrying')",
                "params": []
            }),
            json!({
                "sql": "CREATE INDEX IF NOT EXISTS idx_durable_tasks_recover_leases ON durable_tasks (queue, state, lease_expires_at, created_at) WHERE state = 'leased'",
                "params": []
            }),
            json!({
                "sql": "CREATE INDEX IF NOT EXISTS idx_durable_tasks_recover_retries ON durable_tasks (queue, state, visible_at, created_at) WHERE state = 'retrying'",
                "params": []
            }),
            json!({
                "sql": "CREATE INDEX IF NOT EXISTS idx_durable_tasks_lease_owner ON durable_tasks (lease_id, lease_owner)",
                "params": []
            }),
            json!({
                "sql": "CREATE INDEX IF NOT EXISTS idx_durable_task_events_task_created ON durable_task_events (task_id, created_at)",
                "params": []
            }),
            json!({
                "sql": "CREATE INDEX IF NOT EXISTS idx_durable_dead_letters_created ON durable_dead_letters (created_at)",
                "params": []
            }),
            json!({
                "sql": "CREATE INDEX IF NOT EXISTS idx_durable_idempotency_task_id ON durable_idempotency_keys (task_id)",
                "params": []
            }),
        ],
    );

    openview_queue_database_plan(
        format!(
            "Bootstrap owned OpenView durable queue schema {OPENVIEW_QUEUE_SCHEMA_VERSION} with idempotent table and index migrations"
        ),
        vec![transaction],
    )
}

pub fn iii_database_query_invocation(
    db: impl Into<String>,
    sql: impl Into<String>,
    params: Vec<serde_json::Value>,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "iii-database::query",
        json!({ "db": db.into(), "sql": sql.into(), "params": params }),
    )
}

pub fn iii_database_execute_invocation(
    db: impl Into<String>,
    sql: impl Into<String>,
    params: Vec<serde_json::Value>,
    returning: Vec<impl Into<String>>,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "iii-database::execute",
        json!({
            "db": db.into(),
            "sql": sql.into(),
            "params": params,
            "returning": returning.into_iter().map(Into::into).collect::<Vec<String>>(),
        }),
    )
}

pub fn iii_database_transaction_invocation(
    db: impl Into<String>,
    statements: Vec<serde_json::Value>,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "iii-database::transaction",
        json!({ "db": db.into(), "statements": statements }),
    )
}

pub fn stream_set_invocation(
    stream_name: impl Into<String>,
    group_id: impl Into<String>,
    item_id: impl Into<String>,
    data: serde_json::Value,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "stream::set",
        json!({
            "stream_name": stream_name.into(),
            "group_id": group_id.into(),
            "item_id": item_id.into(),
            "data": data,
        }),
    )
}

pub fn stream_list_invocation(
    stream_name: impl Into<String>,
    group_id: impl Into<String>,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "stream::list",
        json!({
            "stream_name": stream_name.into(),
            "group_id": group_id.into(),
        }),
    )
}

pub fn iii_named_queue_enqueue_invocation(
    function_id: impl Into<String>,
    queue: impl Into<String>,
    payload: serde_json::Value,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(function_id, payload)
        .action(json!({ "kind": "enqueue", "queue": queue.into() }))
}

pub fn iii_durable_publish_invocation(
    topic: impl Into<String>,
    data: serde_json::Value,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "iii::durable::publish",
        json!({ "topic": topic.into(), "data": data }),
    )
}

pub fn openview_queue_task_spawn_plan(
    db: impl Into<String>,
    queue: impl Into<String>,
    task_id: impl Into<String>,
    task_name: impl Into<String>,
    idempotency_key: impl Into<String>,
    payload_ref: impl Into<String>,
    payload_sha256: impl Into<String>,
) -> IiiTriggerPlan {
    let db = db.into();
    let queue = queue.into();
    let task_id = task_id.into();
    let task_name = task_name.into();
    let idempotency_key = idempotency_key.into();
    let payload_ref = payload_ref.into();
    let payload_sha256 = payload_sha256.into();

    let transaction = iii_database_transaction_invocation(
        db,
        vec![
            json!({
                "sql": "INSERT INTO durable_idempotency_keys (idempotency_key, task_id) VALUES (?, ?) ON CONFLICT (idempotency_key) DO NOTHING",
                "params": [idempotency_key, task_id]
            }),
            json!({
                "sql": "INSERT INTO durable_tasks (task_id, queue, task_name, payload_ref, payload_sha256, state, attempt, visible_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'queued', 0, ?, ?, ?) ON CONFLICT (task_id) DO NOTHING",
                "params": [task_id, queue, task_name, payload_ref, payload_sha256, "${now}", "${now}", "${now}"]
            }),
            json!({
                "sql": "INSERT INTO durable_task_events (task_id, kind, payload, created_at) VALUES (?, 'task.spawned', ?, ?)",
                "params": [
                    task_id,
                    {
                        "queue": queue,
                        "task_name": task_name,
                        "payload_ref": payload_ref,
                        "payload_sha256": payload_sha256
                    },
                    "${now}"
                ]
            }),
        ],
    );
    let publish = iii_named_queue_enqueue_invocation(
        "openview.queue::dispatch",
        queue.clone(),
        json!({
            "task_id": task_id,
            "queue": queue,
            "task_name": task_name,
            "payload_ref": payload_ref,
            "payload_sha256": payload_sha256,
        }),
    );

    openview_queue_database_publish_plan(
        "Persist a durable queue task before publishing a dispatch wakeup",
        vec![transaction, publish],
    )
}

pub fn openview_queue_task_claim_lease_plan(
    db: impl Into<String>,
    queue: impl Into<String>,
    worker_id: impl Into<String>,
    lease_id: impl Into<String>,
    lease_expires_at: impl Into<String>,
    limit: u64,
) -> IiiTriggerPlan {
    let db = db.into();
    let queue = queue.into();
    let worker_id = worker_id.into();
    let lease_id = lease_id.into();
    let lease_expires_at = lease_expires_at.into();

    let transaction = iii_database_transaction_invocation(
        db,
        vec![
            json!({
                "sql": "UPDATE durable_tasks SET state = 'queued', lease_id = NULL, lease_owner = NULL, lease_expires_at = NULL, updated_at = ? WHERE queue = ? AND state = 'leased' AND lease_expires_at <= ?",
                "params": ["${now}", queue, "${now}"]
            }),
            json!({
                "sql": "UPDATE durable_tasks SET state = 'leased', lease_id = ?, lease_owner = ?, leased_at = ?, lease_expires_at = ?, updated_at = ? WHERE task_id IN (SELECT task_id FROM durable_tasks WHERE queue = ? AND state IN ('queued', 'retrying') AND visible_at <= ? AND (lease_expires_at IS NULL OR lease_expires_at <= ?) ORDER BY visible_at, created_at LIMIT ?) RETURNING task_id, task_name, payload_ref, payload_sha256, attempt, lease_id, lease_owner, lease_expires_at",
                "params": [lease_id, worker_id, "${now}", lease_expires_at, "${now}", queue, "${now}", "${now}", limit]
            }),
            json!({
                "sql": "INSERT INTO durable_task_events (task_id, kind, payload, created_at) SELECT task_id, 'task.leased', ?, ? FROM durable_tasks WHERE lease_id = ? AND lease_owner = ?",
                "params": [{"lease_id": lease_id, "worker_id": worker_id}, "${now}", lease_id, worker_id]
            }),
        ],
    );

    openview_queue_database_plan(
        "Claim visible durable queue tasks with a time-limited worker lease",
        vec![transaction],
    )
}

pub fn openview_queue_task_heartbeat_lease_plan(
    db: impl Into<String>,
    task_id: impl Into<String>,
    lease_id: impl Into<String>,
    worker_id: impl Into<String>,
    lease_expires_at: impl Into<String>,
) -> IiiTriggerPlan {
    let db = db.into();
    let task_id = task_id.into();
    let lease_id = lease_id.into();
    let worker_id = worker_id.into();
    let lease_expires_at = lease_expires_at.into();

    let heartbeat = iii_database_execute_invocation(
        db,
        "UPDATE durable_tasks SET lease_expires_at = ?, last_heartbeat_at = ?, updated_at = ? WHERE task_id = ? AND lease_id = ? AND lease_owner = ? AND state = 'leased' RETURNING task_id, lease_id, lease_owner, lease_expires_at, last_heartbeat_at",
        vec![json!(lease_expires_at), json!("${now}"), json!("${now}"), json!(task_id), json!(lease_id), json!(worker_id)],
        vec!["task_id", "lease_id", "lease_owner", "lease_expires_at", "last_heartbeat_at"],
    );

    openview_queue_database_plan(
        "Extend a queue task lease only when the current worker still owns it",
        vec![heartbeat],
    )
}

pub fn openview_queue_task_complete_plan(
    db: impl Into<String>,
    task_id: impl Into<String>,
    lease_id: impl Into<String>,
    worker_id: impl Into<String>,
    output_ref: impl Into<String>,
    output_sha256: impl Into<String>,
) -> IiiTriggerPlan {
    let db = db.into();
    let task_id = task_id.into();
    let lease_id = lease_id.into();
    let worker_id = worker_id.into();
    let output_ref = output_ref.into();
    let output_sha256 = output_sha256.into();

    let transaction = iii_database_transaction_invocation(
        db,
        vec![
            json!({
                "sql": "UPDATE durable_tasks SET state = 'completed', output_ref = ?, output_sha256 = ?, completed_at = ?, lease_id = NULL, lease_owner = NULL, lease_expires_at = NULL, updated_at = ? WHERE task_id = ? AND lease_id = ? AND lease_owner = ? AND state = 'leased' RETURNING task_id, output_ref, output_sha256, completed_at",
                "params": [output_ref, output_sha256, "${now}", "${now}", task_id, lease_id, worker_id]
            }),
            json!({
                "sql": "INSERT INTO durable_task_events (task_id, kind, payload, created_at) VALUES (?, 'task.completed', ?, ?)",
                "params": [task_id, {"worker_id": worker_id, "output_ref": output_ref, "output_sha256": output_sha256}, "${now}"]
            }),
        ],
    );
    let publish = iii_durable_publish_invocation(
        "openview.queue.task.completed",
        json!({
            "task_id": task_id,
            "worker_id": worker_id,
            "output_ref": output_ref,
            "output_sha256": output_sha256,
        }),
    );

    openview_queue_database_publish_plan(
        "Complete a leased durable queue task and publish terminal progress",
        vec![transaction, publish],
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OpenViewQueueRetryPlanRequest {
    pub db: String,
    pub queue: String,
    pub task_id: String,
    pub lease_id: String,
    pub worker_id: String,
    pub failure_ref: String,
    pub failure_kind: String,
    pub next_attempt: u64,
    pub retry_at: String,
    pub max_attempts: u64,
}

pub fn openview_queue_task_fail_retry_plan(
    request: OpenViewQueueRetryPlanRequest,
) -> IiiTriggerPlan {
    let OpenViewQueueRetryPlanRequest {
        db,
        queue,
        task_id,
        lease_id,
        worker_id,
        failure_ref,
        failure_kind,
        next_attempt,
        retry_at,
        max_attempts,
    } = request;

    let transaction = iii_database_transaction_invocation(
        db,
        vec![
            json!({
                "sql": "UPDATE durable_tasks SET state = 'retrying', attempt = ?, visible_at = ?, failure_ref = ?, failure_kind = ?, lease_id = NULL, lease_owner = NULL, lease_expires_at = NULL, updated_at = ? WHERE task_id = ? AND lease_id = ? AND lease_owner = ? AND state = 'leased' AND attempt + 1 < ? RETURNING task_id, attempt, visible_at, failure_ref, failure_kind",
                "params": [next_attempt, retry_at, failure_ref, failure_kind, "${now}", task_id, lease_id, worker_id, max_attempts]
            }),
            json!({
                "sql": "INSERT INTO durable_task_events (task_id, kind, payload, created_at) VALUES (?, 'task.retry_scheduled', ?, ?)",
                "params": [task_id, {"worker_id": worker_id, "failure_ref": failure_ref, "failure_kind": failure_kind, "next_attempt": next_attempt, "retry_at": retry_at}, "${now}"]
            }),
        ],
    );
    let publish = iii_named_queue_enqueue_invocation(
        "openview.queue::dispatch",
        queue.clone(),
        json!({
            "task_id": task_id,
            "queue": queue,
            "reason": "retry",
            "attempt": next_attempt,
            "not_before": retry_at,
        }),
    );

    openview_queue_database_publish_plan(
        "Fail a leased task into retry state and publish a retry wakeup",
        vec![transaction, publish],
    )
}

pub fn openview_queue_task_fail_dead_letter_plan(
    db: impl Into<String>,
    task_id: impl Into<String>,
    lease_id: impl Into<String>,
    worker_id: impl Into<String>,
    failure_ref: impl Into<String>,
    failure_kind: impl Into<String>,
) -> IiiTriggerPlan {
    let db = db.into();
    let task_id = task_id.into();
    let lease_id = lease_id.into();
    let worker_id = worker_id.into();
    let failure_ref = failure_ref.into();
    let failure_kind = failure_kind.into();

    let transaction = iii_database_transaction_invocation(
        db,
        vec![
            json!({
                "sql": "UPDATE durable_tasks SET state = 'dead_lettered', failure_ref = ?, failure_kind = ?, dead_lettered_at = ?, lease_id = NULL, lease_owner = NULL, lease_expires_at = NULL, updated_at = ? WHERE task_id = ? AND lease_id = ? AND lease_owner = ? AND state = 'leased' RETURNING task_id, failure_ref, failure_kind, dead_lettered_at",
                "params": [failure_ref, failure_kind, "${now}", "${now}", task_id, lease_id, worker_id]
            }),
            json!({
                "sql": "INSERT INTO durable_dead_letters (task_id, failure_ref, failure_kind, worker_id, created_at) VALUES (?, ?, ?, ?, ?)",
                "params": [task_id, failure_ref, failure_kind, worker_id, "${now}"]
            }),
            json!({
                "sql": "INSERT INTO durable_task_events (task_id, kind, payload, created_at) VALUES (?, 'task.dead_lettered', ?, ?)",
                "params": [task_id, {"worker_id": worker_id, "failure_ref": failure_ref, "failure_kind": failure_kind}, "${now}"]
            }),
        ],
    );
    let publish = iii_durable_publish_invocation(
        "openview.queue.task.dead_lettered",
        json!({
            "task_id": task_id,
            "worker_id": worker_id,
            "failure_ref": failure_ref,
            "failure_kind": failure_kind,
        }),
    );

    openview_queue_database_publish_plan(
        "Fail a leased task into dead-letter state and publish terminal evidence",
        vec![transaction, publish],
    )
}

pub fn openview_queue_recovery_scan_plan(
    db: impl Into<String>,
    queue: impl Into<String>,
    recovery_owner: impl Into<String>,
    limit: u64,
) -> IiiTriggerPlan {
    let db = db.into();
    let queue = queue.into();
    let recovery_owner = recovery_owner.into();

    let query = iii_database_query_invocation(
        db.clone(),
        "SELECT task_id, queue, state, attempt, visible_at, lease_expires_at FROM durable_tasks WHERE queue = ? AND ((state = 'leased' AND lease_expires_at <= ?) OR (state = 'retrying' AND visible_at <= ?)) ORDER BY COALESCE(lease_expires_at, visible_at), created_at LIMIT ?",
        vec![json!(queue), json!("${now}"), json!("${now}"), json!(limit)],
    );
    let execute = iii_database_execute_invocation(
        db,
        "UPDATE durable_tasks SET state = 'queued', lease_id = NULL, lease_owner = NULL, lease_expires_at = NULL, recovered_by = ?, recovered_at = ?, updated_at = ? WHERE queue = ? AND task_id IN (SELECT task_id FROM durable_tasks WHERE queue = ? AND ((state = 'leased' AND lease_expires_at <= ?) OR (state = 'retrying' AND visible_at <= ?)) ORDER BY COALESCE(lease_expires_at, visible_at), created_at LIMIT ?) RETURNING task_id, queue, state, recovered_by",
        vec![
            json!(recovery_owner),
            json!("${now}"),
            json!("${now}"),
            json!(queue),
            json!(queue),
            json!("${now}"),
            json!("${now}"),
            json!(limit),
        ],
        vec!["task_id", "queue", "state", "recovered_by"],
    );
    let publish = iii_named_queue_enqueue_invocation(
        "openview.queue::dispatch",
        queue.clone(),
        json!({
            "queue": queue,
            "recovery_owner": recovery_owner,
            "reason": "recovery_scan",
            "limit": limit,
        }),
    );

    openview_queue_database_publish_plan(
        "Recover expired leases and retry-ready tasks before publishing a dispatch wakeup",
        vec![query, execute, publish],
    )
}

pub fn hook_fanout_publish_collect_invocation(
    topic: impl Into<String>,
    payload: serde_json::Value,
    merge_rule: impl Into<String>,
    timeout_ms: u64,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "hook-fanout::publish_collect",
        json!({
            "topic": topic.into(),
            "payload": payload,
            "merge_rule": merge_rule.into(),
            "timeout_ms": timeout_ms,
        }),
    )
    .timeout_ms(timeout_ms)
}

pub fn approval_resolve_invocation(
    function_call_id: impl Into<String>,
    decision: impl Into<String>,
    reason: Option<impl Into<String>>,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "approval::resolve",
        json!({
            "function_call_id": function_call_id.into(),
            "decision": decision.into(),
            "reason": reason.map(Into::into),
        }),
    )
}

pub fn approval_list_pending_invocation(session_id: impl Into<String>) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "approval::list_pending",
        json!({ "session_id": session_id.into() }),
    )
}

pub fn run_start_invocation(
    session_id: impl Into<String>,
    provider: impl Into<String>,
    model: impl Into<String>,
    messages: Vec<serde_json::Value>,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "run::start",
        json!({
            "session_id": session_id.into(),
            "provider": provider.into(),
            "model": model.into(),
            "messages": messages,
        }),
    )
}

pub fn run_start_and_wait_invocation(
    session_id: impl Into<String>,
    provider: impl Into<String>,
    model: impl Into<String>,
    messages: Vec<serde_json::Value>,
    timeout_ms: u64,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "run::start_and_wait",
        json!({
            "session_id": session_id.into(),
            "provider": provider.into(),
            "model": model.into(),
            "messages": messages,
            "timeout_ms": timeout_ms,
        }),
    )
    .timeout_ms(timeout_ms.saturating_add(5_000))
}

pub fn session_tree_create_invocation(display_name: impl Into<String>) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "session-tree::create",
        json!({ "display_name": display_name.into() }),
    )
}

pub fn session_tree_append_invocation(
    session_id: impl Into<String>,
    message: serde_json::Value,
    parent_id: Option<impl Into<String>>,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "session-tree::append",
        json!({
            "session_id": session_id.into(),
            "message": message,
            "parent_id": parent_id.map(Into::into),
        }),
    )
}

pub fn session_tree_messages_invocation(
    session_id: impl Into<String>,
    limit: Option<u64>,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "session-tree::messages",
        json!({ "session_id": session_id.into(), "limit": limit }),
    )
}

pub fn shell_sandbox_exec_invocation(
    sandbox_id: impl Into<String>,
    command: impl Into<String>,
    args: Vec<impl Into<String>>,
    timeout_ms: Option<u64>,
) -> IiiTriggerInvocation {
    IiiTriggerInvocation::new(
        "shell::exec",
        json!({
            "command": command.into(),
            "args": args.into_iter().map(Into::into).collect::<Vec<String>>(),
            "timeout_ms": timeout_ms,
            "target": { "kind": "sandbox", "sandbox_id": sandbox_id.into() },
        }),
    )
}

pub fn openview_iii_backend_adapter_manifest() -> WorkerManifest {
    let shell_target_schema = json!({
        "type": "object",
        "required": ["kind", "sandbox_id"],
        "properties": {
            "kind": {"type": "string", "enum": ["sandbox"]},
            "sandbox_id": {"type": "string"}
        },
        "additionalProperties": false
    });

    let mut manifest = WorkerManifest::new("openview.iii.backend", "0.1.0")
        .description("Contract-only OpenView adapter for iii primitive worker calls")
        .target(BackendTarget::ThreeICompatible)
        .resource(ResourceKind::Database)
        .resource(ResourceKind::EventStream)
        .resource(ResourceKind::HookTopic)
        .resource(ResourceKind::ApprovalQueue)
        .resource(ResourceKind::SessionState)
        .resource(ResourceKind::Process)
        .resource(ResourceKind::Sandbox)
        .dependency("iii-queue")
        .dependency("iii-database")
        .dependency("hook-fanout")
        .dependency("approval-gate")
        .dependency("turn-orchestrator")
        .dependency("session-tree")
        .dependency("shell")
        .function(
            FunctionSpec::new(OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION, CapabilityRisk::Write)
                .description("Bootstrap owned durable queue table definitions and indexes")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db"],
                    "properties": {
                        "db": {"type": "string"},
                        "schema_version": {"type": "string", "enum": [OPENVIEW_QUEUE_SCHEMA_VERSION]},
                        "migration_id": {"type": "string", "enum": [OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID]}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["schema_version", "migration_id", "tables", "indexes"],
                    "properties": {
                        "schema_version": {"type": "string", "enum": [OPENVIEW_QUEUE_SCHEMA_VERSION]},
                        "migration_id": {"type": "string", "enum": [OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID]},
                        "tables": {"type": "array", "items": {"type": "string"}},
                        "indexes": {"type": "array", "items": {"type": "string"}}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true),
        )
        .function(
            FunctionSpec::new("openview.queue::spawn_task", CapabilityRisk::Write)
                .description("Persist a durable queue task row and publish a named queue dispatch")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "queue", "task_id", "task_name", "idempotency_key", "payload_ref", "payload_sha256"],
                    "properties": {
                        "db": {"type": "string"},
                        "queue": {"type": "string"},
                        "task_id": {"type": "string"},
                        "task_name": {"type": "string"},
                        "idempotency_key": {"type": "string"},
                        "payload_ref": {"type": "string"},
                        "payload_sha256": {"type": "string"}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("openview.queue::dispatch", CapabilityRisk::State)
                .description("Internal named-queue wakeup for leased OpenView task dispatch")
                .request_schema(json!({
                    "type": "object",
                    "required": ["queue"],
                    "properties": {
                        "queue": {"type": "string"},
                        "task_id": {"type": "string"},
                        "task_name": {"type": "string"},
                        "payload_ref": {"type": "string"},
                        "payload_sha256": {"type": "string"},
                        "reason": {"type": "string"},
                        "attempt": {"type": "integer", "minimum": 0},
                        "not_before": {"type": "string"},
                        "recovery_owner": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                }))
                .visibility(FunctionVisibility::Internal),
        )
        .function(
            FunctionSpec::new("openview.queue::claim_lease", CapabilityRisk::Write)
                .description("Claim visible queue tasks with a worker-owned lease")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "queue", "worker_id", "lease_id", "lease_expires_at", "limit"],
                    "properties": {
                        "db": {"type": "string"},
                        "queue": {"type": "string"},
                        "worker_id": {"type": "string"},
                        "lease_id": {"type": "string"},
                        "lease_expires_at": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("openview.queue::heartbeat_lease", CapabilityRisk::Write)
                .description("Extend a queue task lease after matching the current owner")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "task_id", "lease_id", "worker_id", "lease_expires_at"],
                    "properties": {
                        "db": {"type": "string"},
                        "task_id": {"type": "string"},
                        "lease_id": {"type": "string"},
                        "worker_id": {"type": "string"},
                        "lease_expires_at": {"type": "string"}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("openview.queue::complete_task", CapabilityRisk::Write)
                .description("Complete a leased queue task and publish terminal progress")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "task_id", "lease_id", "worker_id", "output_ref", "output_sha256"],
                    "properties": {
                        "db": {"type": "string"},
                        "task_id": {"type": "string"},
                        "lease_id": {"type": "string"},
                        "worker_id": {"type": "string"},
                        "output_ref": {"type": "string"},
                        "output_sha256": {"type": "string"}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("openview.queue::fail_retry", CapabilityRisk::Write)
                .description("Move a leased queue task into retry state and publish a retry wakeup")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "queue", "task_id", "lease_id", "worker_id", "failure_ref", "failure_kind", "next_attempt", "retry_at", "max_attempts"],
                    "properties": {
                        "db": {"type": "string"},
                        "queue": {"type": "string"},
                        "task_id": {"type": "string"},
                        "lease_id": {"type": "string"},
                        "worker_id": {"type": "string"},
                        "failure_ref": {"type": "string"},
                        "failure_kind": {"type": "string"},
                        "next_attempt": {"type": "integer", "minimum": 1},
                        "retry_at": {"type": "string"},
                        "max_attempts": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("openview.queue::fail_dead_letter", CapabilityRisk::Write)
                .description("Move a leased queue task into dead-letter state and publish evidence")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "task_id", "lease_id", "worker_id", "failure_ref", "failure_kind"],
                    "properties": {
                        "db": {"type": "string"},
                        "task_id": {"type": "string"},
                        "lease_id": {"type": "string"},
                        "worker_id": {"type": "string"},
                        "failure_ref": {"type": "string"},
                        "failure_kind": {"type": "string"}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("openview.queue::recovery_scan", CapabilityRisk::Write)
                .description("Recover expired leases and retry-ready rows before publishing dispatch")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "queue", "recovery_owner", "limit"],
                    "properties": {
                        "db": {"type": "string"},
                        "queue": {"type": "string"},
                        "recovery_owner": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("iii-database::query", CapabilityRisk::Read)
                .description("Read rows through iii-database::query")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "sql"],
                    "properties": {
                        "db": {"type": "string"},
                        "sql": {"type": "string"},
                        "params": {"type": "array"},
                        "timeout_ms": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("iii-database::execute", CapabilityRisk::Write)
                .description("Write SQL through iii-database::execute")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "sql"],
                    "properties": {
                        "db": {"type": "string"},
                        "sql": {"type": "string"},
                        "params": {"type": "array"},
                        "returning": {"type": "array", "items": {"type": "string"}}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true),
        )
        .function(
            FunctionSpec::new("iii-database::transaction", CapabilityRisk::Write)
                .description("Atomic iii-database transaction plan")
                .request_schema(json!({
                    "type": "object",
                    "required": ["db", "statements"],
                    "properties": {
                        "db": {"type": "string"},
                        "statements": {"type": "array", "items": {"type": "object"}},
                        "isolation": {"type": "string"}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true),
        )
        .function(
            FunctionSpec::new("iii::durable::publish", CapabilityRisk::Stream)
                .description("Publish a topic message through iii-queue durable pub/sub"),
        )
        .function(
            FunctionSpec::new("stream::set", CapabilityRisk::Stream)
                .description("Append or replace a value on an iii stream")
                .request_schema(json!({
                    "type": "object",
                    "required": ["stream_name", "group_id", "item_id", "data"],
                    "properties": {
                        "stream_name": {"type": "string"},
                        "group_id": {"type": "string"},
                        "item_id": {"type": "string"},
                        "data": {}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("stream::list", CapabilityRisk::Read)
                .description("List values from an iii stream")
                .request_schema(json!({
                    "type": "object",
                    "required": ["stream_name", "group_id"],
                    "properties": {
                        "stream_name": {"type": "string"},
                        "group_id": {"type": "string"}
                    },
                    "additionalProperties": false
                })),
        )
        .function(
            FunctionSpec::new("hook-fanout::publish_collect", CapabilityRisk::Stream)
                .description("Publish a hook topic and collect subscriber replies"),
        )
        .function(
            FunctionSpec::new("approval::resolve", CapabilityRisk::Approval)
                .description("Resolve a pending approval gate entry")
                .approval_required(true),
        )
        .function(
            FunctionSpec::new("approval::list_pending", CapabilityRisk::Read)
                .description("List pending approval gate entries"),
        )
        .function(
            FunctionSpec::new("run::start", CapabilityRisk::Exec)
                .description("Start a durable iii agent run")
                .approval_required(true),
        )
        .function(
            FunctionSpec::new("run::start_and_wait", CapabilityRisk::Exec)
                .description("Start a durable iii agent run and wait for terminal output")
                .approval_required(true),
        )
        .function(
            FunctionSpec::new("session-tree::create", CapabilityRisk::State)
                .description("Create a session-tree session"),
        )
        .function(
            FunctionSpec::new("session-tree::append", CapabilityRisk::State)
                .description("Append a message to a session-tree session"),
        )
        .function(
            FunctionSpec::new("session-tree::messages", CapabilityRisk::Read)
                .description("Read messages from a session-tree session"),
        )
        .function(
            FunctionSpec::new("shell::exec", CapabilityRisk::Exec)
                .description("Execute a shell command through the sandbox target handoff")
                .request_schema(json!({
                    "type": "object",
                    "required": ["command", "target"],
                    "properties": {
                        "command": {"type": "string"},
                        "args": {"type": "array", "items": {"type": "string"}},
                        "timeout_ms": {"type": ["integer", "null"], "minimum": 1},
                        "target": shell_target_schema,
                    },
                    "additionalProperties": false
                }))
                .approval_required(true),
        )
        .function(
            FunctionSpec::new("sandbox::exec", CapabilityRisk::Exec)
                .description("Direct sandbox exec handoff when shell::exec is unavailable")
                .approval_required(true),
        );

    manifest.language = "rust".to_string();
    manifest.deploy_kind = "contract".to_string();
    manifest.default_config = json!({
        "network_required": false,
        "transport": "iii.trigger",
        "contract_only": true,
        "queue_action": "TriggerAction.Enqueue({ queue })",
        "queue_dispatch_function": "openview.queue::dispatch",
        "queue_lease_owner": "openview.iii.backend",
        "queue_schema_version": OPENVIEW_QUEUE_SCHEMA_VERSION,
        "queue_schema_migration_function": OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION,
        "queue_schema_migration_id": OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID,
        "queue_tables": OPENVIEW_QUEUE_TABLE_NAMES,
        "queue_indexes": OPENVIEW_QUEUE_INDEX_NAMES,
        "queue_schema_migrations": [
            {
                "id": OPENVIEW_QUEUE_SCHEMA_MIGRATION_ID,
                "version": OPENVIEW_QUEUE_SCHEMA_VERSION,
                "function": OPENVIEW_QUEUE_SCHEMA_BOOTSTRAP_FUNCTION,
                "tables": OPENVIEW_QUEUE_TABLE_NAMES,
                "indexes": OPENVIEW_QUEUE_INDEX_NAMES
            }
        ],
        "shell_handoff": "shell::exec target=sandbox; fallback sandbox::exec",
    });
    manifest
}

pub fn runtime_manifest_json(manifest: &WorkerManifest) -> Result<String, WorkerManifestError> {
    Ok(serde_json::to_string_pretty(manifest)?)
}

pub fn approval_worker_manifest() -> WorkerManifest {
    WorkerManifest::new("approval.gate", "0.1.0")
        .description("Human approval queue for risky agent actions")
        .resource(ResourceKind::ApprovalQueue)
        .function(FunctionSpec::new(
            "approval::request",
            CapabilityRisk::Approval,
        ))
        .function(FunctionSpec::new(
            "approval::resolve",
            CapabilityRisk::Approval,
        ))
}

pub fn shell_sandbox_worker_manifest() -> WorkerManifest {
    WorkerManifest::new("shell.sandbox", "0.1.0")
        .description("Sandboxed command execution with explicit policy and approval")
        .resource(ResourceKind::Sandbox)
        .resource(ResourceKind::Process)
        .function(FunctionSpec::new("shell::plan", CapabilityRisk::Read))
        .function(FunctionSpec::new("shell::run", CapabilityRisk::Exec).approval_required(true))
}

pub fn hermes_agent_worker_manifest() -> WorkerManifest {
    const WORKSPACE_ROOT: &str = "${OPENVIEW_WORKSPACE_ROOT}";
    const HERMES_BINARY: &str = "hermes";

    let mut manifest = WorkerManifest::new("hermes.agent", "0.1.0")
        .description("Hermes Agent session lifecycle, prompt delivery, event streaming, and approval resolution")
        .target(BackendTarget::LocalBinary)
        .resource(ResourceKind::SessionState)
        .resource(ResourceKind::EventStream)
        .resource(ResourceKind::ApprovalQueue)
        .resource(ResourceKind::Filesystem)
        .resource(ResourceKind::Process)
        .resource(ResourceKind::CredentialVault)
        .resource(ResourceKind::ModelProvider)
        .function(
            FunctionSpec::new("hermes.agent::spawn_session", CapabilityRisk::Exec)
                .description("Start or resume a Hermes Agent session in a scoped workspace")
                .request_schema(json!({
                    "type": "object",
                    "required": ["profile", "prompt", "workspace"],
                    "properties": {
                        "profile": {"type": "string", "description": "Hermes profile name passed with --profile"},
                        "prompt": {"type": "string", "description": "Initial user prompt or task instruction"},
                        "workspace": {"type": "string", "description": "Workspace directory under the OpenView workspace root"},
                        "session_id": {"type": ["string", "null"], "description": "Optional Hermes session id to resume"},
                        "skills": {"type": "array", "items": {"type": "string"}, "default": []},
                        "toolsets": {"type": "array", "items": {"type": "string"}, "default": []},
                        "approval_policy": {"type": "string", "enum": ["ask", "yolo"], "default": "ask"}
                    },
                    "additionalProperties": false
                }))
                .response_schema(json!({
                    "type": "object",
                    "required": ["session_id", "started", "event_stream"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "started": {"type": "boolean"},
                        "event_stream": {"type": "string"},
                        "pid": {"type": ["integer", "null"]},
                        "approval_queue": {"type": ["string", "null"]}
                    },
                    "additionalProperties": false
                }))
                .approval_required(true)
                .visibility(FunctionVisibility::Ui),
        )
        .function(
            FunctionSpec::new("hermes.agent::send_prompt", CapabilityRisk::State)
                .description("Send a follow-up prompt to an existing Hermes session")
                .request_schema(json!({
                    "type": "object",
                    "required": ["session_id", "prompt"],
                    "properties": {
                        "session_id": {"type": "string"},
                        "prompt": {"type": "string"}
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
            FunctionSpec::new("hermes.agent::stream_events", CapabilityRisk::Read)
                .description("Read the event stream for a Hermes session")
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
            FunctionSpec::new("hermes.agent::resolve_approval", CapabilityRisk::Approval)
                .description("Approve or deny a pending Hermes tool approval")
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
        .sandbox(
            SandboxProfile::locked_down()
                .allow_workspace_read(WORKSPACE_ROOT)
                .allow_workspace_write(WORKSPACE_ROOT)
                .allow_command(HERMES_BINARY),
        );

    manifest.binary_name = Some(HERMES_BINARY.to_string());
    manifest.language = "python".to_string();
    manifest.deploy_kind = "local_cli".to_string();
    manifest.default_config = json!({
        "binary": HERMES_BINARY,
        "env_keys": ["HERMES_PROFILE", "HERMES_HOME"],
        "session_id_source": "hermes_session_id",
        "event_stream": "openview.worker.hermes.agent.events",
        "approval_queue": "openview.worker.hermes.agent.approvals",
        "approval_policy": "ask"
    });
    manifest
}
