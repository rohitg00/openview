import type { RunInput, RunRecord, WorkerDefinition } from "./types.js";

export interface FunctionBackend {
  readonly name: string;
  registerWorker(worker: WorkerDefinition): Promise<void>;
  startRun(input: RunInput): Promise<{ runId: string }>;
  waitForRun(runId: string): Promise<RunRecord>;
}

export interface ThreeICompatibleBackendOptions {
  endpoint: string;
  token?: string;
  namespace?: string;
}

export class ThreeICompatibleBackend implements FunctionBackend {
  readonly name = "3i-compatible";
  constructor(private readonly options: ThreeICompatibleBackendOptions) {}

  async registerWorker(worker: WorkerDefinition): Promise<void> {
    await this.post("/workers/register", worker);
  }

  async startRun(input: RunInput): Promise<{ runId: string }> {
    return this.post("/runs/start", input) as Promise<{ runId: string }>;
  }

  async waitForRun(runId: string): Promise<RunRecord> {
    return this.post("/runs/wait", { runId }) as Promise<RunRecord>;
  }

  private async post(path: string, body: unknown): Promise<unknown> {
    const response = await fetch(new URL(path, this.options.endpoint), {
      method: "POST",
      headers: {
        "content-type": "application/json",
        ...(this.options.token ? { authorization: `Bearer ${this.options.token}` } : {}),
        ...(this.options.namespace ? { "x-openview-namespace": this.options.namespace } : {}),
      },
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      throw new Error(`Backend request failed: ${response.status} ${response.statusText}`);
    }
    return response.json();
  }
}
