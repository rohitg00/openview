# OpenView Architecture

OpenView is an orchestration layer with a local-first core and a backend-ready boundary.

## Goals

1. Compose many agents without forcing them into one giant prompt.
2. Keep execution narrow: agents decide, workers execute, tools expose capabilities.
3. Make side effects explicit through permissions and approvals.
4. Make every run observable through structured events.
5. Keep backend integration small enough to map to a 3i-compatible engine.

## Runtime layers

### 1. Registry

The registry stores agents, workers, and tools.

- Agents define behavior and permissions.
- Workers define capabilities, schemas, health, runtime, side effects, and operational hints.
- Tools bind a worker capability to a callable interface.

### 2. Run engine

The local run engine executes a list of agents sequentially today. The model is deliberately simple so it can be extended into graphs, fan-out/fan-in, loops, and human steering.

Each run stores:

- stable `run.id`;
- goal and input;
- mutable state;
- output;
- steps;
- approvals;
- event timeline.

### 3. Approval layer

Permissions are checked before each agent step. Any permission with `approval: "required"` blocks the run and creates an approval request.

A blocked run can be resumed with `approveAndResume(runId, approvalId, reason)`.

### 4. Event layer

Events are monotonic and cursor-friendly. UI and backend streams can resume from a cursor using `eventsAfter(runId, cursor)`.

Important event types:

- `run.started`
- `agent.started`
- `agent.message`
- `approval.requested`
- `approval.approved`
- `run.blocked`
- `agent.completed`
- `run.completed`
- `run.failed`

### 5. Backend adapter

The backend adapter is intentionally narrow:

```ts
interface FunctionBackend {
  registerWorker(worker): Promise<void>;
  startRun(input): Promise<{ runId: string }>;
  waitForRun(runId): Promise<RunRecord>;
}
```

A 3i-compatible backend can own durability, worker registration, queues, function invocation, and run streams while OpenView keeps the user-facing model stable.

## Extension points

Future extension points should not break the core primitives:

- graph execution plans;
- run cancellation;
- artifact store;
- trace exporters;
- worker manifests;
- policy evaluator;
- dashboard stream transport;
- backend-owned resume/replay.
