import { createOpenView, permission } from "../core/index.js";
import type { AgentDefinition, RunInput, WorkerDefinition } from "../core/index.js";

export interface OpenViewClientOptions {
  endpoint?: string;
}

export class OpenViewClient {
  private readonly local = createOpenView();
  constructor(private readonly options: OpenViewClientOptions = {}) {}

  registerAgent(agent: AgentDefinition): AgentDefinition {
    return this.local.registerAgent(agent);
  }

  registerWorker(worker: WorkerDefinition): WorkerDefinition {
    return this.local.registerWorker(worker);
  }

  async run(input: RunInput) {
    if (!this.options.endpoint) return this.local.startRun(input);
    const response = await fetch(new URL("/runs/start", this.options.endpoint), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(input),
    });
    if (!response.ok) throw new Error(`OpenView server returned ${response.status}`);
    return response.json();
  }
}

export { permission };
export type { AgentDefinition, RunInput, WorkerDefinition };
