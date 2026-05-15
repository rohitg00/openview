# OpenView

OpenView is an iii-native control plane for running coding agents as real
workers.

It is built for the workflow operators actually want: launch Claude Code,
Codex, Hermes, OpenClaw, OpenCode, Gemini CLI, Goose, Aider, OpenHands, Qwen
Code, Cursor Agent, Amp, and other agent CLIs side-by-side; keep each one in its
own git worktree; route risky actions through approvals; stream every event;
and persist enough state to recover, audit, and hand work off.

OpenView is not trying to be another prompt box. It is the worker/runtime layer
for AgentView: agents decide, narrow workers execute, humans approve, iii
primitives provide durability, and every run leaves evidence.

## Why It Exists

Agent CLIs are powerful, but they are usually operated as invisible local
processes. That breaks down when a team wants to run several agents at once,
compare results, resume work, review approvals, and understand exactly what
happened.

OpenView gives that workflow a typed control plane:

- **Multi-agent workspaces:** one view for many agents, many repos, and many
  worktrees.
- **Worker-first execution:** every agent runner is an iii function-call worker,
  not a hard-coded UI special case.
- **Approval gates:** file writes, shell commands, credential use, network
  access, and other risky actions can block for review.
- **Durable orchestration:** queues, leases, retries, dead letters, streams,
  sessions, and database state are mapped to iii primitives.
- **Inspectable evidence:** runs expose events, approvals, worker health, queue
  state, artifacts, and review bundles.

## What You Can Do Today

The Rust workspace already includes:

- worker manifests and invocation contracts for the AgentView runner catalog;
- a core run state machine with approvals, events, graph compilation, resources,
  sandbox policy, and worker registry types;
- queue/task contracts for leases, visibility timeouts, retries, recovery,
  requeue, completion, and dead-letter state;
- a local durable store for runs, approvals, events, workers, queue tasks,
  artifacts, restart snapshots, and evidence summaries;
- CLI surfaces for catalog, manifests, approvals, worker status, run events,
  queue readiness, cancellation, and evidence export;
- a live iii E2E script that checks harness health, models, streams,
  session-tree, approvals, durable publish, hook fanout, REST bridge, shell
  execution, and optional iii-database task-state durability.

The honest status: this repo has the control-plane contracts, local runtime
model, CLI demos, and live iii verification path. Full product parity still
requires the polished AgentView UI, published runner worker packages for every
agent, and production-grade live runner adapters behind the same contracts.

## Quickstart

Clone and verify the local Rust surface:

```bash
git clone https://github.com/rohitg00/openview.git
cd openview
bash scripts/dev_check.sh
```

Or run the core commands manually:

```bash
cargo test --workspace
cargo run -p openview-cli -- catalog
cargo run -p openview-cli -- manifest hermes.agent
cargo run -p openview-cli -- demo
cargo run -p openview-cli -- queues readiness
```

The CLI commands above do not need credentials. They use deterministic local
data so developers can review the worker catalog, approval flow, queue
readiness, and evidence shape before connecting a live iii runtime.

## Install iii Workers

OpenView's install story is intentionally iii-first. Operators should add
workers through the iii worker manager, not copy manifests by hand.

Install iii:

```bash
curl -fsSL https://install.iii.dev/iii/main/install.sh | sh
```

Install the OpenView substrate workers:

```bash
bash scripts/install_iii_workers.sh --core-only
```

That installs the shared runtime surface OpenView expects:

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
iii worker add git-worktree
iii worker add iii-database
iii worker add iii-queue
iii worker add iii-sandbox
iii worker add harness
```

Then install the AgentView runner wrappers you want:

```bash
bash scripts/install_iii_workers.sh --agents-only
```

Equivalent manual commands:

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

The wrapper is the iii worker package. The upstream agent CLI is still installed
separately and configured by path. That keeps OpenView honest: the control plane
owns policy, worktrees, approvals, streams, and durable state; the upstream tool
keeps owning its model-specific behavior.

Preview the full install plan without changing your machine:

```bash
bash scripts/install_iii_workers.sh --dry-run
```

## Configure Credentials

Start iii and store model credentials through the credential worker:

```bash
iii start
iii trigger --function-id auth::set_token --payload '{"provider":"anthropic","credential":{"type":"api_key","key":"sk-ant-..."}}'
iii trigger --function-id auth::set_token --payload '{"provider":"openai","credential":{"type":"api_key","key":"sk-..."}}'
```

OpenView manifests reference credential names and environment key names. They
should not store raw secrets.

## AgentView Runner Model

Each runner worker exposes the same lifecycle shape even when the underlying CLI
uses different flags:

1. create or select a `git.worktree`;
2. launch the agent CLI through the shell/process sandbox;
3. route risky actions through `approval.gate`;
4. write stdout, stderr, tool calls, approvals, and status updates to iii
   streams;
5. persist transcripts and branches in `session-tree`;
6. store run/task state in `iii-database` and `iii-queue`;
7. emit evidence that a human can review.

Current runner manifests:

| Worker | Local binary | Purpose |
|---|---|---|
| `codex.agent` | `codex` | Codex CLI sessions in isolated worktrees. |
| `claude-code.agent` | `claude` | Claude Code sessions in isolated worktrees. |
| `hermes.agent` | `hermes` | Hermes profile/session runs with skills, toolsets, approvals, and events. |
| `openclaw.agent` | `openclaw` | OpenClaw sessions through the shared runner contract. |
| `opencode.agent` | `opencode` | OpenCode sessions through AgentView. |
| `gemini-cli.agent` | `gemini` | Gemini CLI sessions with normalized streams. |
| `goose.agent` | `goose` | Goose sessions scoped to worktrees. |
| `aider.agent` | `aider` | Aider coding runs behind approval gates. |
| `openhands.agent` | `openhands` | OpenHands sessions through the shared runner wrapper. |
| `crush.agent` | `crush` | Crush runs with replayable output streams. |
| `qwen-code.agent` | `qwen` | Qwen Code sessions through AgentView. |
| `cursor-agent.agent` | `cursor-agent` | Cursor Agent sessions as worker runs. |
| `amp.agent` | `amp` | Amp sessions as worker runs. |

Inspect any manifest:

```bash
cargo run -p openview-cli -- manifest codex.agent
cargo run -p openview-cli -- manifest claude-code.agent
cargo run -p openview-cli -- manifest hermes.agent
cargo run -p openview-cli -- manifest openclaw.agent
```

## Live iii Runtime Check

For a real runtime check, run the E2E script:

```bash
OPENVIEW_III_MANAGED=1 OPENVIEW_III_E2E_DATABASE=1 bash scripts/e2e_iii_runtime.sh
```

What it verifies:

- harness worker health;
- model catalog exposes GPT-5 with xhigh support;
- stream write/read round trip;
- session-tree create/append/messages round trip;
- approval queue shape;
- durable publish path;
- hook fanout publish/collect;
- REST bridge invocation;
- shell sandbox execution;
- optional iii-database schema bootstrap, lease heartbeat, expired-lease
  recovery, retry, and dead-letter behavior.

Set these when your local iii paths differ:

```bash
III_BIN=/path/to/iii
III_WORKERS_REPO=/path/to/iii-workers
III_DEMO_SCRIPT=/path/to/harness/scripts/demo.sh
III_E2E_DEMO_DIR=/private/tmp/openview-iii-e2e
```

You can also run the live check through cargo:

```bash
OPENVIEW_RUN_LIVE_III_E2E=1 cargo test -p openview-worker --test live_iii_e2e -- --nocapture
```

## Architecture

```text
User goal
  |
  v
AgentView
  |-- planner / builder / reviewer agents
  |-- one git worktree per run
  |-- approvals, traces, events, terminals, artifacts
  v
OpenView control plane
  |-- worker registry and manifests
  |-- run state machine
  |-- approval state
  |-- sandbox policy
  |-- queue and durable store contracts
  v
iii primitives
  |-- iii-queue for delivery, leases, retries, dead letters
  |-- iii-database for durable state and fallback transactions
  |-- streams for live panes and replay
  |-- session-tree for transcripts and branches
  |-- approval workers for human gates
  |-- shell/process sandbox for execution
  |-- harness for local UI/event streaming
```

OpenView keeps the contracts typed in Rust and maps the runtime to iii workers.
That makes the system inspectable before it is distributed.

## Developer Workflow

Use the repo check script for the normal loop:

```bash
bash scripts/dev_check.sh
```

Use strict mode before pushing larger changes:

```bash
OPENVIEW_STRICT=1 bash scripts/dev_check.sh
```

Use live mode when changing iii adapter behavior:

```bash
OPENVIEW_RUN_LIVE_III=1 OPENVIEW_III_MANAGED=1 OPENVIEW_III_E2E_DATABASE=1 bash scripts/dev_check.sh
```

Manual commands:

```bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/check_public_copy.sh
```

## Crates

```text
crates/openview-core    Rust contracts, run state, registry, graph, store, policy
crates/openview-worker  Worker manifests and runner invocation contracts
crates/openview-cli     Operator CLI for catalog, demos, readiness, and evidence
```

## Docs

- `docs/workers.md` explains worker manifests, resources, runner wrappers, and
  iii primitive mapping.
- `docs/control-plane.md` explains AgentView panes, workspace sessions, worker
  runtime lifecycle, and operator CLI parity.
- `docs/architecture.md` captures the broader system shape.
- `docs/plans/iii-adapter-parity.md` tracks iii adapter parity.
- `docs/plans/parity-acceptance-checklist.md` tracks the parity acceptance
  checklist.

## Roadmap

1. Publish/install runner worker packages through `iii worker add`.
2. Replace deterministic CLI shims with durable APIs backed by the iii adapter.
3. Ship the AgentView UI for side-by-side agents, worktrees, traces, approvals,
   terminals, files, and evidence.
4. Add production restart, retry, cancellation, and output persistence drills for
   live runners.
5. Harden worker marketplace metadata, versioning, health checks, and upgrade
   flow.

## Public-Copy Rule

Public docs should describe OpenView directly. Do not leak private research
notes, private repo names, or competitor-specific implementation notes into
public code or docs. Run:

```bash
bash scripts/check_public_copy.sh
```
