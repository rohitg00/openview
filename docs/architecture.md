# OpenView Architecture

OpenView is a Rust-first agent-worker orchestrator. It is designed around explicit resources, explicit capabilities, and explicit backend boundaries.

## Layers

### 1. Agent graph

An agent graph describes who decides, who executes, who reviews, and where approvals happen. The graph compiles into required workers, handoff edges, shared resources, and execution mode.

### 2. Worker registry

The registry stores worker manifests. Each manifest declares:

- worker identity and version;
- Rust binary deployment metadata;
- backend targets;
- functions and trigger specs;
- resources;
- dependencies;
- default config;
- security policy;
- sandbox profile.

### 3. Run state machine

Runs move through phases:

- queued
- provisioning
- running
- waiting for approval
- function calling
- streaming
- completed
- failed
- cancelled

The foundation implementation includes approval blocking and resume. The next implementation step is durable step persistence and replay.

### 4. Event stream

Every run emits ordered events. This becomes the transport for a future dashboard, CLI watch mode, mobile approval surface, and backend stream bridge.

### 5. Backend boundary

OpenView emits worker/function/trigger registration messages. It does not vendor backend internals. The backend can provide process supervision, WebSocket transport, queueing, retry, state, stream groups, and observability.

## Worker connection patterns

### Approval-protected shell worker

```text
agent -> policy.guard -> approval.gate -> shell.sandbox -> event stream
```

### Durable agent turn

```text
run.start -> turn.orchestrator -> models.router -> function call hooks -> storage/session state
```

### Review loop

```text
commit event -> repo worker -> planner -> builder -> reviewer -> CI worker -> hardening report
```

## Non-goals for the foundation

- No copied runtime code from private references.
- No direct changes to external backend or worker repositories.
- No hidden network/process access.
- No default write permissions.
