import type { AgentDefinition, ToolDefinition, WorkerDefinition } from "./types.js";

export class OpenViewRegistry {
  readonly agents = new Map<string, AgentDefinition>();
  readonly workers = new Map<string, WorkerDefinition>();
  readonly tools = new Map<string, ToolDefinition>();

  registerAgent(agent: AgentDefinition): AgentDefinition {
    assertUnique(this.agents, agent.id, "agent");
    this.agents.set(agent.id, agent);
    return agent;
  }

  registerWorker(worker: WorkerDefinition): WorkerDefinition {
    assertUnique(this.workers, worker.id, "worker");
    this.workers.set(worker.id, worker);
    return worker;
  }

  registerTool(tool: ToolDefinition): ToolDefinition {
    if (!this.workers.has(tool.workerId)) {
      throw new Error(`Cannot register tool ${tool.id}: missing worker ${tool.workerId}`);
    }
    assertUnique(this.tools, tool.id, "tool");
    this.tools.set(tool.id, tool);
    return tool;
  }
}

function assertUnique<T>(map: Map<string, T>, id: string, kind: string): void {
  if (map.has(id)) {
    throw new Error(`Duplicate ${kind} id: ${id}`);
  }
}
