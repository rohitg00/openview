# OpenView

**Agent View. Open View.** OpenView is a small, typed orchestration layer for composing agents, workers, tools, approvals, and live run traces into one developer-friendly control plane.

OpenView is built for teams that want to run many specialized agents without turning their product into an opaque chat loop. Agents decide, workers execute, tools declare capabilities, approvals protect side effects, and every run emits a timeline you can inspect, resume, and harden.

## Why OpenView exists

AI teams are moving from “one agent with a prompt” to fleets of narrow workers: code readers, file editors, browser runners, repo reviewers, deployment checkers, data fetchers, memory tools, notification senders, and human approval gates. The hard part is no longer calling a model. The hard part is coordinating work safely:

- Which agent owns the next decision?
- Which worker is allowed to touch the filesystem, network, shell, or secrets?
- What happened before the run got stuck?
- Can a human approve one step without restarting the whole task?
- Can the same flow run locally now and on a durable backend later?

OpenView answers those questions with a tiny set of primitives and a clear event trail.

## Core model

OpenView keeps the vocabulary intentionally small:

- **Agent** — a decision-making role with instructions, permissions, tools, and a handler.
- **Worker** — a narrow capability provider with schemas, side-effect class, health, and runtime metadata.
- **Tool** — a callable capability exposed by a worker.
- **Run** — one user goal moving through one or more agents.
- **Step** — one agent execution inside a run.
- **Approval** — a human decision required before risky side effects.
- **Event** — a structured timeline item for UI, logs, traces, replay, and debugging.
- **Backend** — a local or 3i-compatible function engine that can register workers and execute runs.

This lets OpenView stay useful as a local TypeScript library while leaving room for durable execution, remote workers, registries, dashboards, and mobile approvals.

## Install

```bash
npm install openview
```

For local development from this repository:

```bash
npm install
npm test
npm run typecheck
```

## First useful run

```ts
import { createOpenView, permission } from "openview";

const view = createOpenView();

view.registerAgent({
  id: "planner",
  name: "Planner",
  instructions: "Turn a user goal into scoped execution steps.",
  permissions: [permission.readOnly()],
  handler: async ({ input }) => ({ plan: [`Research ${input.goal}`, `Build ${input.goal}`] }),
});

view.registerAgent({
  id: "executor",
  name: "Executor",
  instructions: "Execute one scoped task at a time.",
  permissions: [permission.readOnly()],
  handler: async ({ state }) => ({ completed: state.plan }),
});

const run = await view.startRun({
  goal: "a safe multi-agent workflow",
  agents: ["planner", "executor"],
});

console.log(run.status);       // completed
console.log(run.steps.length); // 2
console.log(run.events);       // structured timeline
```

## Approval gates by default

OpenView treats side effects as explicit product decisions. A read-only agent can run immediately. A writer, shell runner, network caller, or secret reader can require approval before execution.

```ts
view.registerAgent({
  id: "workspace-writer",
  name: "Workspace Writer",
  instructions: "Prepare a file change after approval.",
  permissions: [permission.writeWorkspace({ paths: ["docs/decision.md"], approval: "required" })],
  handler: async () => ({ artifact: "docs/decision.md", summary: "change prepared" }),
});

const blocked = await view.startRun({ goal: "prepare a decision note", agents: ["workspace-writer"] });
console.log(blocked.status); // blocked

const completed = await view.approveAndResume(blocked.id, blocked.approvals[0].id, "approved after review");
console.log(completed.status); // completed
```

## Worker registry

Workers are not anonymous tools. They are typed, inspectable capability providers.

```ts
view.registerWorker({
  id: "repo.reader",
  name: "Repository Reader",
  version: "0.1.0",
  runtime: "local",
  capabilities: ["files.read", "code.search"],
  sideEffects: "read-only",
  inputSchema: { type: "object", required: ["query"] },
  outputSchema: { type: "object", required: ["matches"] },
  health: "online",
});
```

A registry entry should tell operators and developers:

- what the worker can do;
- what inputs and outputs it accepts;
- what it can mutate;
- whether approval is needed;
- how healthy it is;
- what cost, latency, and failure modes to expect;
- which runtime surface owns it: local, remote, serverless, browser, mobile, desktop, MCP, or function backend.

## Architecture

```text
User goal
   │
   ▼
OpenView run
   │
   ├─ Agent step: plan / route / decide
   │     └─ emits events
   │
   ├─ Approval gate when a permission requires it
   │     └─ approve, deny, retry, or cancel
   │
   ├─ Worker/tool execution through local or backend adapter
   │     └─ typed input/output and side-effect metadata
   │
   └─ Completed run with steps, approvals, output, and event timeline
```

Current package layout:

```text
packages/core    Typed primitives, registry, permissions, event store, local run engine
packages/sdk     Thin client wrapper for local or remote use
examples         Runnable examples for basic orchestration and approvals
docs             Architecture, API, worker design, and implementation notes
tests            Node test runner coverage for state, events, approvals, registry
```

## Backend direction

OpenView starts local because local is the best way to design good developer experience. The backend interface is intentionally narrow so it can map to a 3i-compatible engine later:

```ts
interface FunctionBackend {
  registerWorker(worker): Promise<void>;
  startRun(input): Promise<{ runId: string }>;
  waitForRun(runId): Promise<RunRecord>;
}
```

That separation lets the same OpenView flow run in three modes:

1. **Local mode** — fast SDK tests, demos, and prototypes.
2. **Hybrid mode** — local planner with remote workers.
3. **Durable mode** — backend-owned functions, streams, worker heartbeats, retries, and resumable runs.

## Product principles

OpenView is designed from three points of view:

### For users

- Show every active run in one place.
- Make it obvious when an agent is thinking, using a worker, waiting, blocked, or done.
- Give humans clear approval cards with what will happen and why.
- Let users interrupt or resume work without losing the thread.

### For developers

- Code-first API with a clear local test story.
- Small primitives instead of framework magic.
- Worker manifests that are easy to review.
- Event timelines that make bugs reproducible.
- No hidden side effects: permissions live next to the agent or tool using them.

### For operators and founders

- Agent work becomes observable work, not mystery output.
- Worker capabilities become reusable product surface area.
- Approval, audit, and trace data are part of the core model.
- The same product can grow from local SDK to durable backend without a rewrite.

## Research-backed design inputs

OpenView’s shape comes from current public signals around agent infrastructure:

- durable workers and functions are becoming the substrate for long-running agent work;
- registries need capability metadata, schemas, health, side effects, and versioning;
- approvals, sandboxing, and default-deny permissions are table stakes;
- observability needs semantic spans for decisions, tools, handoffs, memory, state, and artifacts;
- users expect to start, watch, interrupt, approve, and resume agent sessions across surfaces.

The public README intentionally describes the product pattern rather than naming individual systems. See `docs/research-notes.md` for the distilled, source-neutral research summary used to shape the implementation.

## Examples

```bash
npm run build
node dist/examples/basic-agent.js
node dist/examples/approval-gate.js
```

## CLI

The CLI is intentionally tiny in the foundation release. It gives developers a fast way to smoke-test the local run engine and inspect worker metadata before wiring a dashboard or durable backend.

```bash
npm run build
node dist/packages/cli/index.js demo:basic
node dist/packages/cli/index.js demo:approval
node dist/packages/cli/index.js inspect:worker
```

## Roadmap

- [x] Local typed orchestration core
- [x] Worker registry metadata
- [x] Approval gates and resume
- [x] Event timeline with resume cursors
- [x] 3i-compatible backend adapter shape
- [ ] Web dashboard for the Agent View
- [ ] Worker manifest validator
- [ ] Event stream transport
- [ ] Durable backend integration
- [ ] Mobile approval surface
- [ ] OpenTelemetry/OpenMetrics export
- [ ] Policy-as-code permission evaluator

## Development

```bash
npm install
npm test
npm run typecheck
npm run lint:copy
```

The copy check prevents accidental references to private research targets in public source/docs.

## License

MIT
