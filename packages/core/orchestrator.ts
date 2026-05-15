import { createEventStore } from "./events.js";
import { permission, requiresApproval } from "./permissions.js";
import { OpenViewRegistry } from "./registry.js";
import type {
  AgentDefinition,
  ApprovalRequest,
  JsonObject,
  OpenViewEvent,
  RunInput,
  RunRecord,
  RunStep,
  WorkerDefinition,
  ToolDefinition,
} from "./types.js";

export interface OpenViewApp {
  readonly registry: OpenViewRegistry;
  registerAgent(agent: AgentDefinition): AgentDefinition;
  registerWorker(worker: WorkerDefinition): WorkerDefinition;
  registerTool(tool: ToolDefinition): ToolDefinition;
  startRun(input: RunInput): Promise<RunRecord>;
  approveAndResume(runId: string, approvalId: string, reason?: string): Promise<RunRecord>;
  getRun(runId: string): RunRecord | undefined;
  eventsAfter(runId: string, cursor: number): OpenViewEvent[];
}

export function createOpenView(): OpenViewApp {
  const registry = new OpenViewRegistry();
  const runs = new Map<string, RunRecord>();
  let nextRun = 0;
  let nextStep = 0;
  let nextApproval = 0;

  const app: OpenViewApp = {
    registry,
    registerAgent(agent) {
      return registry.registerAgent(agent);
    },
    registerWorker(worker) {
      return registry.registerWorker(worker);
    },
    registerTool(tool) {
      return registry.registerTool(tool);
    },
    async startRun(input) {
      nextRun += 1;
      const runId = `run_${nextRun.toString(36)}`;
      const now = new Date().toISOString();
      const eventStore = createEventStore();
      const run: RunRecord = {
        id: runId,
        goal: input.goal,
        status: "running",
        input: { ...(input.input ?? {}), goal: input.goal },
        state: { ...(input.state ?? {}) },
        steps: [],
        approvals: [],
        events: [],
        createdAt: now,
        updatedAt: now,
      };
      const emit = (type: Parameters<typeof eventStore.append>[0], payload: JsonObject) => {
        const event = eventStore.append(type, payload);
        run.events = eventStore.all();
        run.updatedAt = event.timestamp;
        return event;
      };
      emit("run.started", { runId, goal: input.goal, agents: input.agents });
      runs.set(runId, run);
      return executeAgents(run, input.agents, emit);
    },
    async approveAndResume(runId, approvalId, reason = "approved") {
      const run = mustGetRun(runs, runId);
      const approval = run.approvals.find((item) => item.id === approvalId);
      if (!approval) throw new Error(`Approval not found: ${approvalId}`);
      approval.status = "approved";
      approval.reason = reason;
      approval.resolvedAt = new Date().toISOString();
      const eventStore = createEventStore(run.events);
      const emit = (type: Parameters<typeof eventStore.append>[0], payload: JsonObject) => {
        const event = eventStore.append(type, payload);
        run.events = eventStore.all();
        run.updatedAt = event.timestamp;
        return event;
      };
      emit("approval.approved", { runId, approvalId, reason });
      const remaining = run.steps.filter((step) => step.status === "waiting_for_approval").map((step) => step.agentId);
      run.status = "running";
      return executeAgents(run, remaining, emit);
    },
    getRun(runId) {
      return runs.get(runId);
    },
    eventsAfter(runId, cursor) {
      return mustGetRun(runs, runId).events.filter((event) => event.sequence > cursor);
    },
  };

  async function executeAgents(
    run: RunRecord,
    agentIds: string[],
    emit: (type: Parameters<ReturnType<typeof createEventStore>["append"]>[0], payload: JsonObject) => OpenViewEvent,
  ): Promise<RunRecord> {
    try {
      for (const agentId of agentIds) {
        const agent = registry.agents.get(agentId);
        if (!agent) throw new Error(`Unknown agent: ${agentId}`);
        const existingWaiting = run.steps.find((step) => step.agentId === agentId && step.status === "waiting_for_approval");
        const step = existingWaiting ?? makeStep(run.id, agentId);
        if (!existingWaiting) run.steps.push(step);
        const approval = requiresApproval(agent.permissions);
        const alreadyApproved = approval
          ? run.approvals.some((item) => item.agentId === agentId && item.permission.mode === approval.mode && item.status === "approved")
          : true;
        if (approval && !alreadyApproved) {
          nextApproval += 1;
          const request: ApprovalRequest = {
            id: `appr_${nextApproval.toString(36)}`,
            runId: run.id,
            agentId,
            permission: approval,
            status: "pending",
            createdAt: new Date().toISOString(),
          };
          run.approvals.push(request);
          step.status = "waiting_for_approval";
          run.status = "blocked";
          emit("approval.requested", { runId: run.id, approvalId: request.id, agentId, permission: approval });
          emit("run.blocked", { runId: run.id, reason: "approval_required", agentId });
          return run;
        }
        step.status = "running";
        step.startedAt = new Date().toISOString();
        emit("agent.started", { runId: run.id, agentId });
        const output = await agent.handler({
          run,
          step,
          input: run.input as JsonObject & { goal: string },
          state: run.state,
          emit,
        });
        step.output = output;
        step.status = "completed";
        step.completedAt = new Date().toISOString();
        run.output = output;
        if (isRecord(output)) {
          run.state = { ...run.state, ...output };
        }
        emit("agent.completed", { runId: run.id, agentId, output: isRecord(output) ? output : { value: output } });
      }
      run.status = "completed";
      emit("run.completed", { runId: run.id, output: isRecord(run.output) ? run.output : { value: run.output } });
      return run;
    } catch (error) {
      run.status = "failed";
      emit("run.failed", { runId: run.id, error: error instanceof Error ? error.message : String(error) });
      return run;
    }
  }

  function makeStep(runId: string, agentId: string): RunStep {
    nextStep += 1;
    return { id: `step_${nextStep.toString(36)}`, runId, agentId, status: "queued" };
  }

  return app;
}

function mustGetRun(runs: Map<string, RunRecord>, runId: string): RunRecord {
  const run = runs.get(runId);
  if (!run) throw new Error(`Run not found: ${runId}`);
  return run;
}

function isRecord(value: unknown): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export { permission };
