# OpenView

**Agent View. Open View.** OpenView is a Rust-first control plane for composing agents, narrowly scoped workers, approval gates, sandbox policies, resources, and backend-compatible function execution.

The goal is not to be another thin agent wrapper. OpenView is a product and runtime blueprint for coordinating real work: agents decide, workers execute, policies constrain, humans approve, and every run leaves an inspectable event trail.

## What OpenView is

OpenView is a small orchestrator with a large systems shape:

- **Rust core** for manifests, registry, graph compilation, run state, approvals, sandbox policy, and backend messages.
- **Worker catalog** for approval, shell sandbox, turn orchestration, model routing, policy, storage, state, streams, budgets, hooks, credentials, MCP, terminals, and worktrees.
- **Backend-compatible protocol boundary** that can register workers, functions, and triggers without vendoring another engine.
- **CLI** for inspecting worker manifests and running local demos.
- **Docs-first DevX** so founders, operators, developers, and users understand how workers connect before anything becomes magic.

## Why Rust

OpenView is infrastructure. The worker surface is process-oriented, permission-sensitive, and expected to run across local machines, remote workers, and durable backends. Rust is the right default because it gives us:

- typed contracts for worker manifests and schemas;
- explicit state machines for long-running runs;
- safer filesystem/process/network boundaries;
- reliable CLI/binary distribution;
- direct alignment with Rust worker ecosystems;
- predictable tests and CI.

## Mental model

```text
User goal
  │
  ▼
Agent graph
  ├─ planner agent
  ├─ builder agent
  └─ reviewer agent
  │
  ▼
Worker registry
  ├─ approval.gate        human decisions
  ├─ shell.sandbox        scoped command execution
  ├─ turn.orchestrator    durable agent turn state
  ├─ models.router        model/provider routing
  ├─ policy.guard         denylist and permission evaluation
  ├─ storage.objects      artifacts and payloads
  ├─ session.state        durable state resources
  ├─ hooks.fanout         before/after tool-call hooks
  └─ mcp.bridge           external tool/server import
  │
  ▼
Backend boundary
  ├─ register worker
  ├─ register function
  ├─ register trigger
  ├─ invoke/enqueue
  └─ stream events
```

## Worker resources

OpenView does not treat workers as generic labels. A worker manifest declares the resources it owns or needs:

- `Sandbox` — isolated execution environment.
- `Filesystem` — read/write roots and denied paths.
- `Network` — allowed hosts or no-network mode.
- `Process` — allowed command execution.
- `CredentialVault` — named credentials without exposing values.
- `ModelProvider` and `ModelRouter` — provider calls and routing.
- `ApprovalQueue` — human approval requests and resolutions.
- `SessionState` — durable transcript/run/session state.
- `EventStream` — live run and worker events.
- `ObjectStorage` — artifacts, logs, screenshots, payloads.
- `Database` — structured state or app data.
- `HookTopic` — before/after function-call hooks.
- `BudgetLedger` — token/cost/rate budget checks.
- `PolicySet` — denylist and capability policy.
- `Terminal` — interactive command sessions.
- `GitWorktree` — branch/worktree scoped agent work.

## Sandbox and permission model

The default policy is deny. A worker must declare what it can do.

```rust
let sandbox = SandboxProfile::locked_down()
    .allow_workspace_read("/workspace")
    .deny_network();

let manifest = WorkerManifest::new("shell.sandbox", "0.1.0")
    .function(FunctionSpec::new("shell::run", CapabilityRisk::Exec).approval_required(true))
    .resource(ResourceKind::Sandbox)
    .sandbox(sandbox);
```

A risky function can block a run until a human approves it. The approval record stores the function, worker, reason, resolution reason, and event trail.

## Backend-compatible messages

OpenView emits a minimal registration shape:

- `registerworker`
- `registerfunction`
- `registertrigger`

This keeps OpenView independent while allowing a durable function/worker backend to own queueing, long-running processes, streams, and replay.

## Current crates

```text
crates/openview-core    Rust types, worker registry, graph compiler, run state machine
crates/openview-worker  Binary worker manifest helpers and sample worker manifests
crates/openview-cli     CLI for catalog, manifest, and local demo flows
```

## Quickstart

```bash
cargo test --workspace
cargo run -p openview-cli -- catalog
cargo run -p openview-cli -- manifest shell.sandbox
cargo run -p openview-cli -- demo
```

## Tests

The test suite covers the core contract:

- worker manifests include functions, resources, security policy, and sandbox profile;
- agent graphs connect planner/builder/reviewer with approval gates and shared resources;
- run state blocks on approval and resumes to completed;
- backend-compatible registration messages are emitted without depending on backend internals.

## Roadmap

1. **Foundation:** Rust core, worker manifests, graph compiler, approval state machine.
2. **Worker runtime:** binary worker process runner, manifest validator, health checks.
3. **Sandbox runner:** filesystem/process/network policy enforcement for local commands.
4. **Durable backend adapter:** WebSocket registration, invocation, queueing, event streams.
5. **Agent View UI:** sessions, runs, workers, approvals, traces, worktrees, terminals.
6. **Worker marketplace/catalog:** documented worker resources and safe install flows.
7. **Review loop:** every run produces evidence about what improved, what broke, and what needs hardening.

## Development

```bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/check_public_copy.sh
```

## Public-copy rule

The implementation can be informed by private research, but public code, commits, and docs should describe OpenView directly. The copy guard prevents restricted reference names from leaking into the repository.
