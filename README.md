# OpenView

**Agent View. Open View.** OpenView is a Rust-first control plane for composing agents, narrowly scoped workers, approval gates, sandbox policies, resources, and backend-compatible function execution.

The goal is not to be another thin agent wrapper. OpenView is a product and runtime blueprint for coordinating real work: agents decide, workers execute, policies constrain, humans approve, and every run leaves an inspectable event trail.

## What OpenView is

OpenView is a small orchestrator with a large systems shape:

- **Rust core** for manifests, registry, graph compilation, run state, approvals, sandbox policy, and backend messages.
- **Worker catalog** for approval, shell sandbox, turn orchestration, model routing, policy, storage, state, streams, budgets, hooks, credentials, MCP, terminals, worktrees, and AgentView runner workers for agent CLIs such as Codex, Claude Code, Hermes, OpenClaw, OpenCode, Gemini CLI, Goose, Aider, OpenHands, Crush, Qwen Code, Cursor Agent, and Amp.
- **Backend-compatible protocol boundary** that can register workers, functions, and triggers without vendoring another engine.
- **iii primitive adapter target** for queue-backed tasks, database transactions, event streams, approvals, session history, shell sandbox execution, and harness-backed UI/event streaming.
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

## AgentView runner workers

OpenView now models local coding agents as first-class workers instead of loose
commands in a text box:

| Worker | Local binary | Role |
|---|---|---|
| `codex.agent` | `codex` | Non-interactive Codex CLI runs in isolated worktrees. |
| `claude-code.agent` | `claude` | Claude Code print-mode sessions in isolated worktrees. |
| `hermes.agent` | `hermes` | Hermes profile/session runs with skills, toolsets, approvals, and events. |
| `openclaw.agent` | `openclaw` | OpenClaw runs with the same worktree/session contract. |
| `opencode.agent` | `opencode` | OpenCode runs in isolated worktrees. |
| `gemini-cli.agent` | `gemini` | Gemini CLI runs with normalized events and approvals. |
| `goose.agent` | `goose` | Goose sessions wrapped as function-call worker runs. |
| `aider.agent` | `aider` | Aider coding sessions scoped to a selected worktree. |
| `openhands.agent` | `openhands` | OpenHands CLI/session runs behind the AgentView contract. |
| `crush.agent` | `crush` | Crush agent runs with replayable output streams. |
| `qwen-code.agent` | `qwen` | Qwen Code CLI runs through the shared runner contract. |
| `cursor-agent.agent` | `cursor-agent` | Cursor Agent CLI runs through AgentView. |
| `amp.agent` | `amp` | Amp agent runs through AgentView. |

Every runner is wrapped as an iii function-call worker. The runner contract is
the same even when the underlying CLI flags differ: create or select a
`git.worktree`, launch the CLI through the shell/process sandbox, route risky
actions through `approval.gate`, write ordered output to iii streams, persist
history in `session-tree`, store durable task/run state in `iii-database` and
`iii-queue`, and resolve credentials/models through the configured credential
and model-provider workers.

AgentView should then show several agents side-by-side, each with its own
workspace/worktree, approval queue, event stream, and session history. The Rust
code exposes concrete manifest and invocation contracts for the full runner
catalog above. That is not the same as complete production runtime parity: each
runner still needs its upstream CLI installed and live adapter checks for its
package before operators should mark it production-ready.

The DevEx install path is always the iii worker manager. Install the shared
substrate first:

```bash
iii worker add shell
iii worker add git-worktree
iii worker add approval-gate
iii worker add session-tree
iii worker add iii-database
iii worker add iii-queue
iii worker add auth-credentials
iii worker add provider-router
iii worker add subagent
iii worker add harness
```

Then add the runner workers you want AgentView to offer. Published worker
packages should follow the same explicit `iii worker add ...` flow:

```bash
iii worker add codex-agent
iii worker add claude-code-agent
iii worker add hermes-agent
iii worker add openclaw-agent
iii worker add opencode-agent
iii worker add gemini-cli-agent
iii worker add goose-agent
iii worker add aider-agent
iii worker add openhands-agent
iii worker add crush-agent
iii worker add qwen-code-agent
iii worker add cursor-agent
iii worker add amp-agent
```

The agent runner binaries remain local CLI tools referenced by worker manifests;
`iii worker add` installs/configures the wrapper, not the upstream CLI itself.
Verify the catalog contracts that are implemented in this repo with:

```bash
cargo run -p openview-cli -- manifest codex.agent
cargo run -p openview-cli -- manifest claude-code.agent
cargo run -p openview-cli -- manifest hermes.agent
cargo run -p openview-cli -- manifest opencode.agent
```

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

The current runtime target is iii primitive parity rather than standalone durability first. OpenView should map tasks/runs/checkpoints/events/waits/idempotency/retry/cancel onto `iii-queue` when available, `iii-database` transactions as durable fallback, `stream::set` and `stream::list` for ordered events, `hook-fanout::publish_collect` for hooks, `approval::resolve` and `approval::list_pending` for human gates, `run::start_and_wait` for Hermes agents, `session-tree::*` for transcript/history, shell sandbox for execution, and harness for UI/event streaming. See `docs/plans/iii-adapter-parity.md`.

## Current runtime status

OpenView now owns the local queue/store contract surfaces:

- `QueueTask` and `OpenViewControlApi` cover enqueue, compatible polling, lease ownership, visibility timeout, stale lease rejection, complete/fail, retry backoff, terminal failed/dead-letter state, explicit requeue, and expired-lease recovery.
- `LocalRunStore` covers workflow definitions, run snapshots, run events, approvals, worker runtimes, queue tasks, artifacts, restart snapshots, current queue lease state, and evidence bundle summaries.
- Worker heartbeat is modeled in runtime supervision; `OpenViewControlApi::heartbeat_worker_leases` extends active owned queue leases from a worker heartbeat; and the process supervisor now records a sanitized heartbeat event that names that control API call-site.
- The live iii E2E database mode applies the owned durable queue schema bootstrap, verifies durable tables/indexes/columns, and exercises database-backed task lease, heartbeat, recovery, retry, and dead-letter state. Full production workflow parity still requires the live runner/API/UX surfaces, task output persistence, graph advancement, and end-to-end restart/retry/cancellation drills.
- The CLI exposes `queues readiness` as a deterministic, secret-free queue adapter snapshot covering the queue schema/migration surface, worker heartbeat-to-lease call-site status, concurrency policy observability, and live iii database E2E lifecycle.

## Install the iii worker stack

OpenView's iii-backed runtime expects the supporting workers to be installed
through the iii worker manager, not copied by hand. On a fresh machine, install
the iii CLI first:

```bash
curl -fsSL https://install.iii.dev/iii/main/install.sh | sh
```

Then add the worker stack OpenView targets:

```bash
iii worker add auth-credentials
iii worker add models-catalog
iii worker add provider-anthropic
iii worker add provider-openai
iii worker add provider-router
iii worker add turn-orchestrator
iii worker add session-tree
iii worker add session-inbox
iii worker add hook-fanout
iii worker add policy-denylist
iii worker add approval-gate
iii worker add llm-budget
iii worker add skills
iii worker add shell
iii worker add subagent
iii worker add iii-database
iii worker add iii-sandbox
iii worker add harness
```

Optional OAuth helper workers:

```bash
iii worker add oauth-anthropic
iii worker add oauth-openai-codex
```

`iii worker add` writes worker config into `~/.iii/config.yaml`; `iii start`
then starts the configured workers. Store model credentials through the
credential worker:

```bash
iii start
iii trigger --function-id auth::set_token --payload '{"provider":"anthropic","credential":{"type":"api_key","key":"sk-ant-..."}}'
iii trigger --function-id auth::set_token --payload '{"provider":"openai","credential":{"type":"api_key","key":"sk-..."}}'
```

For local development before every worker has a registry release, OpenView's
live E2E can run against the local iii-hq worker checkout:

```bash
OPENVIEW_III_MANAGED=1 OPENVIEW_III_E2E_DATABASE=1 scripts/e2e_iii_runtime.sh
```

That managed path starts the same harness worker stack from the configured
`III_WORKERS_REPO`; the production install path remains `iii worker add`.

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
cargo run -p openview-cli -- approvals list
cargo run -p openview-cli -- approvals approve-demo
cargo run -p openview-cli -- approvals reject-demo
cargo run -p openview-cli -- workers status
cargo run -p openview-cli -- queues readiness
cargo run -p openview-cli -- runs watch
cargo run -p openview-cli -- runs cancel-demo
cargo run -p openview-cli -- evidence export-demo
```

The operator-facing commands above are deterministic demo-runtime shims. They expose the current approval queue, approval resolution, approval rejection to `run.failed`, run cancellation to `run.cancelled`, worker health, queue adapter readiness, run events, and a review evidence bundle without requiring external services or storing secret values.

For a real iii runtime check against the local engine and harness workers, run:

```bash
scripts/e2e_iii_runtime.sh
```

Set `OPENVIEW_III_MANAGED=1` to have the script start and stop the local iii demo stack. Set `OPENVIEW_III_E2E_DATABASE=1` as well to start `iii-database`, apply the owned durable queue schema bootstrap, and verify transactional task-state persistence against SQLite. The script exercises harness health, model catalog, streams, session-tree, approvals, durable publish, hook fanout, REST bridge, shell execution, and the optional database schema/lease/heartbeat/recovery/retry/dead-letter path.

The default SQLite path for the live database-backed task-state check is `$III_E2E_DEMO_DIR/openview-e2e.db`, which resolves to `/private/tmp/openview-iii-e2e/openview-e2e.db` unless overridden.

## Tests

The test suite covers the core contract:

- worker manifests include functions, resources, security policy, and sandbox profile;
- agent graphs connect planner/builder/reviewer with approval gates and shared resources;
- run state blocks on approval and resumes to completed;
- backend-compatible registration messages are emitted without depending on backend internals;
- workspace sessions model tabs, panes, worktrees, traces, and approvals;
- worker runtime specs track binary lifecycle and health without storing secret values;
- worker runtime supervision records start, heartbeat, failure, restart, and stop events;
- queue contracts cover task enqueue, compatible polling, lease ownership, visibility timeout, stale lease rejection, completion, retry, requeue, dead-letter terminal state, and expired-lease recovery;
- durable store contracts cover workflow definitions, runs, events, queue tasks, approvals, worker runtimes, artifacts, restart snapshots, and evidence bundle summaries;
- sandbox policy evaluation checks read/write roots, denied roots, command allowlists, network hosts, and secret-name references without touching the host system;
- the gated live iii E2E script can be run from cargo with `OPENVIEW_RUN_LIVE_III_E2E=1 cargo test -p openview-worker --test live_iii_e2e -- --nocapture`.

## Roadmap

1. **Foundation:** Rust core, worker manifests, graph compiler, approval state machine.
2. **Process-backed worker runtime:** binary worker process runner, manifest validator, health checks.
3. **Sandbox runner:** filesystem/process/network policy enforcement for local commands.
4. **iii primitive adapter:** queue-backed tasks through iii-queue when available, iii-database transaction fallback, stream events, approval gates, session-tree history, shell sandbox execution, and harness-backed UI/event streaming.
5. **Agent View UI:** sessions, runs, workers, approvals, traces, worktrees, terminals. See `docs/control-plane.md`.
6. **Worker marketplace/catalog:** documented worker resources and safe install flows.
7. **Review loop:** every run produces evidence about what improved, what broke, and what needs hardening.

## Development

```bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/check_public_copy.sh
scripts/e2e_iii_runtime.sh
```

## Public-copy rule

The implementation can be informed by private research, but public code, commits, and docs should describe OpenView directly. The copy guard prevents restricted reference names from leaking into the repository.
