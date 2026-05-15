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

Health states:

- `unknown`
- `starting`
- `ready`
- `degraded`
- `stopped`
- `failed`

## Next implementation tranche

1. Add persistent storage for workspace sessions.
2. Add worker runtime supervisor trait.
3. Add terminal/PTY worker manifest and tests.
4. Add git worktree worker manifest and tests.
5. Add event-stream cursor and replay API.
6. Add approval queue listing/filtering API.
7. Add backend WebSocket adapter behind a trait.
