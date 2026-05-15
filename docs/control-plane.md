# Control Plane

OpenView needs a visible control plane because orchestration is not just function calls. Users need to see what is running, which worker owns each resource, where approvals are blocked, and what evidence exists for a run.

## Durable workspace session

A workspace session is the restorable UI/control-plane shape for a project or operational room.

It contains:

- tabs;
- panes;
- worktree bindings;
- restored run ids;
- timestamps for migration and backup.

The current Rust foundation includes:

- `WorkspaceSession`
- `WorkspaceTab`
- `WorkspacePane`
- `PaneKind`
- `WorktreeBinding`
- `WorkerRuntimeSpec`
- `WorkerRuntimeSupervisor`
- `WorkerRuntimeEvent`

## Pane kinds

- `AgentRun` — run-level view of planner/builder/reviewer flow.
- `Terminal` — terminal/PTY pane owned by a terminal worker.
- `Editor` — source or artifact view.
- `Browser` — browser/session pane for web tasks.
- `Notes` — run notes, decisions, and operator context.
- `ApprovalQueue` — pending approvals and resolved decisions.
- `WorkerCatalog` — installed workers, manifests, health, and capabilities.
- `TraceTimeline` — ordered event stream and function-call lifecycle.
- `Worktree` — branch/worktree scoped engineering state.

## Why this matters

Without a control plane, agents become invisible background jobs. OpenView should make the system inspectable:

```text
workspace
  ├─ tab: Runs
  │   ├─ pane: Trace timeline
  │   └─ pane: Approval queue
  ├─ tab: Build
  │   ├─ pane: Terminal
  │   └─ pane: Worktree
  └─ tab: Workers
      ├─ pane: Worker catalog
      └─ pane: Policy set
```

## Worker runtime lifecycle

A worker runtime spec tracks how a worker binary is launched and monitored:

- worker id;
- binary name/path;
- args;
- environment variable names only, never secret values;
- working directory;
- health state;
- last heartbeat timestamp.
- failure reason and timestamp;
- restart count;
- ordered runtime lifecycle events.

Health states:

- `unknown`
- `starting`
- `ready`
- `degraded`
- `stopped`
- `failed`

## iii primitive-backed runtime target

The control plane should read from OpenView APIs that are backed by an iii adapter, not from ad hoc UI state. The target mapping is:

- `iii-queue` owns task delivery, retry, concurrency, FIFO, and dead-letter behavior when available.
- `iii-database` is the durable transaction fallback for runs, tasks, checkpoints, idempotency keys, waits, approvals, and event rows.
- `stream::set` writes live run/worker/approval events; `stream::list` replays ordered slices for trace panes.
- `hook-fanout::publish_collect` fans before/after function-call hooks to policy, approval, budget, and evidence subscribers.
- `approval::list_pending` and `approval::resolve` back the approval queue pane.
- `run::start_and_wait` is the adapter target for Hermes agent turns.
- `session-tree::*` stores transcript/history so a run can be resumed or exported.
- shell sandbox workers execute scoped commands only after OpenView policy allows them.
- harness composes the UI/event streaming path for local operator surfaces.

Local live check: `scripts/e2e_iii_runtime.sh` uses a configured `III_BIN` path and verifies the running iii engine plus harness workers without requiring `iii` to be on `PATH`.

## Operator CLI parity shims

The current CLI exposes deterministic demo-runtime shims for the operator surfaces needed by the control plane:

```bash
cargo run -p openview-cli -- approvals list
cargo run -p openview-cli -- approvals approve-demo
cargo run -p openview-cli -- approvals reject-demo
cargo run -p openview-cli -- workers status
cargo run -p openview-cli -- queues readiness
cargo run -p openview-cli -- control-plane queue-readiness
cargo run -p openview-cli -- runs watch
cargo run -p openview-cli -- runs events
cargo run -p openview-cli -- runs cancel-demo
cargo run -p openview-cli -- evidence export-demo
```

These commands use in-memory demo data from the Rust runtime. They do not read credentials, emit secret values, or mutate project files. The rejection shim resolves the demo approval and emits `approval.rejected` plus `run.failed`; the cancellation shim emits `run.cancelled` for the demo run. The queue readiness shim reports deterministic status for the queue schema/bootstrap surface, control API heartbeat-to-lease wiring, process-supervisor heartbeat call-site, concurrency policy observability, and live iii database E2E lifecycle while marking production adapter binding as still incomplete. The evidence export includes run events, approval state, worker runtime health, queue adapter readiness, review readiness, and placeholder artifact slots so humans can review the expected bundle shape before durable storage is implemented.

## Next implementation tranche

1. Add persistent storage for workspace sessions.
2. Add process-backed worker runtime supervisor implementation.
3. Add terminal/PTY worker process implementation.
4. Add git worktree worker process implementation.
5. Replace the deterministic CLI parity shims with durable approval, worker, run, and evidence APIs.
6. Add backend WebSocket adapter behind a trait.
