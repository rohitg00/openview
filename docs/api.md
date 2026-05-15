# API Reference

## `createOpenView()`

Creates a local OpenView app with an in-memory registry, run store, approval store, and event store.

```ts
const view = createOpenView();
```

## `registerAgent(agent)`

Registers a decision-making agent.

Required fields:

- `id`
- `name`
- `instructions`
- `permissions`
- `handler`

Handlers receive `{ run, step, input, state, emit }` and return any serializable output. Object outputs are merged into run state to make sequential handoffs easy.

## `registerWorker(worker)`

Registers a capability provider.

Worker metadata should describe runtime, capability scope, side-effect class, schemas, and health. This gives UI and policy layers enough information to explain risk before a tool runs.

## `startRun(input)`

Starts a run.

```ts
await view.startRun({
  goal: "review a repository",
  agents: ["planner", "reader", "reviewer"],
});
```

The current foundation executes agents sequentially. Future graph support should preserve the same run, step, approval, and event model.

## `approveAndResume(runId, approvalId, reason)`

Approves a blocked permission request and resumes waiting agent steps.

## `eventsAfter(runId, cursor)`

Returns monotonic events after a cursor. This is the basis for web, desktop, mobile, and CLI surfaces that reconnect to a long-running run.

## Permission helpers

- `permission.readOnly(resources?)`
- `permission.writeWorkspace({ paths, approval })`
- `permission.network({ hosts, approval })`
- `permission.process({ commands, approval })`
