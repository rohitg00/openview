export type JsonObject = Record<string, unknown>;

export type RunStatus =
  | "queued"
  | "running"
  | "blocked"
  | "completed"
  | "failed"
  | "cancelled";

export type StepStatus =
  | "queued"
  | "running"
  | "waiting_for_approval"
  | "completed"
  | "failed"
  | "skipped";

export type PermissionMode = "read" | "write" | "network" | "process" | "secret";
export type ApprovalMode = "none" | "required";

export interface PermissionScope {
  mode: PermissionMode;
  description: string;
  approval: ApprovalMode;
  resources?: string[];
}

export interface ApprovalRequest {
  id: string;
  runId: string;
  agentId: string;
  permission: PermissionScope;
  status: "pending" | "approved" | "denied";
  reason?: string;
  createdAt: string;
  resolvedAt?: string;
}

export type EventType =
  | "run.started"
  | "run.blocked"
  | "run.completed"
  | "run.failed"
  | "agent.started"
  | "agent.message"
  | "agent.completed"
  | "agent.failed"
  | "approval.requested"
  | "approval.approved"
  | "approval.denied"
  | "worker.registered";

export interface OpenViewEvent {
  id: string;
  sequence: number;
  type: EventType;
  timestamp: string;
  payload: JsonObject;
}

export interface RunStep {
  id: string;
  runId: string;
  agentId: string;
  status: StepStatus;
  startedAt?: string;
  completedAt?: string;
  output?: unknown;
  error?: string;
}

export interface RunRecord {
  id: string;
  goal: string;
  status: RunStatus;
  input: JsonObject;
  state: JsonObject;
  output?: unknown;
  steps: RunStep[];
  approvals: ApprovalRequest[];
  events: OpenViewEvent[];
  createdAt: string;
  updatedAt: string;
}

export interface RunInput {
  goal: string;
  agents: string[];
  input?: JsonObject;
  state?: JsonObject;
}

export interface AgentContext {
  run: RunRecord;
  step: RunStep;
  input: JsonObject & { goal: string };
  state: JsonObject;
  emit: (type: EventType, payload: JsonObject) => OpenViewEvent;
}

export type AgentHandler = (context: AgentContext) => Promise<unknown> | unknown;

export interface AgentDefinition {
  id: string;
  name: string;
  instructions: string;
  permissions: PermissionScope[];
  handler: AgentHandler;
  tools?: string[];
  workerHints?: string[];
}

export type WorkerRuntime = "local" | "remote" | "serverless" | "browser" | "mobile" | "desktop" | "mcp" | "function";
export type SideEffectClass = "read-only" | "writes-files" | "network" | "executes-code" | "external-side-effect";
export type HealthStatus = "online" | "offline" | "degraded" | "unknown";

export interface WorkerDefinition {
  id: string;
  name: string;
  version: string;
  runtime: WorkerRuntime;
  capabilities: string[];
  sideEffects: SideEffectClass;
  inputSchema: JsonObject;
  outputSchema: JsonObject;
  health: HealthStatus;
  costHint?: string;
  latencyHint?: string;
  failureModes?: string[];
  approvalRequired?: boolean;
}

export interface ToolDefinition {
  id: string;
  name: string;
  workerId: string;
  capability: string;
  inputSchema: JsonObject;
  outputSchema: JsonObject;
  permission: PermissionScope;
}
