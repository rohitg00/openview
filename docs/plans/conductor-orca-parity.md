# Conductor and Spinnaker Pipeline Parity Roadmap

## Objective

OpenView should grow from a Rust contract foundation into an inspectable agent-worker control plane that can handle durable workflows, deployment-style runbooks, approvals, sandboxed execution, and parallel agent work.

This roadmap compares OpenView against two public references:

- Conductor-style durable workflow engines: task queues, polling workers, retries, state history, APIs, search, and pluggable persistence.
- Spinnaker-style pipeline engines: stage graphs, task graphs inside stages, manual judgment, execution history, auditability, and remediation loops for infrastructure operations.

The goal is not to clone either system. The goal is parity on the operator guarantees that matter for agentic work:

1. every unit of work has typed inputs, typed outputs, and owned resources;
2. every risky action is policy-checked before execution;
3. every run can pause, resume, retry, fail, cancel, and leave evidence;
4. every worker can be supervised, scaled, inspected, and replaced;
5. every human approval has a durable reason, resolution, and audit trail.

## Current OpenView status

OpenView already has the right shape for a Rust-first orchestration foundation.

Implemented or documented today:

- Rust workspace with `openview-core`, `openview-worker`, and `openview-cli`.
- Worker manifests with functions, triggers, resources, deployment targets, security policy, and sandbox profile.
- Built-in worker catalog covering approval, shell sandbox, turn orchestration, model routing, policy, storage, terminal PTY, and git worktree capabilities.
- Agent graph compilation for planner, builder, reviewer, shared resources, approval gates, and durable queued execution mode.
- Run state machine that can block on approval and resume to completed.
- Backend-compatible registration messages for worker, function, and trigger registration.
- Workspace session model for tabs, panes, worktrees, traces, terminals, approvals, and worker catalog views.
- Worker runtime specs and supervision events for start, heartbeat, failure, restart, and stop.
- Sandbox policy evaluation for filesystem roots, denied roots, exact command identities, network hosts, and secret-name references.
- Queue lease/retry/recovery contracts for enqueue, compatible polling, lease ownership, visibility timeout, current-lease complete/fail, stale lease rejection, retry backoff, terminal failed/dead-letter state, explicit requeue, and expired-lease reclamation.
- Local durable store contracts for workflow definitions, run snapshots, run events, approvals, worker runtimes, queue tasks, artifacts, restart snapshots, and evidence bundle summaries.
- Live iii E2E database-backed task-state check through `iii-database::transaction`, `iii-database::query`, and `iii-database::execute` when `OPENVIEW_III_E2E_DATABASE=1` is enabled.
- CLI queue adapter readiness snapshot through `openview queues readiness`, covering schema/migration ownership, worker heartbeat-to-lease call-site status, concurrency policy observability, and live iii database E2E lifecycle state.
- Public-copy guard to keep implementation docs focused on OpenView terms.

What this means:

OpenView currently proves the contract and local runtime surfaces for queue/store/recovery ownership. It does not yet provide the production runtime that executes workflows end to end through live process workers, enforced sandbox execution, and operator UI/API services.

## Parity map

| Capability | Conductor-style expectation | Spinnaker-style expectation | OpenView status | Gap |
|---|---|---|---|---|
| Workflow definition | Workflows define task order, task types, inputs, outputs, and state transitions. | Pipelines define ordered and parallel stages. Each stage expands into tasks. | Agent graphs compile planner, builder, reviewer, approvals, and shared resources. | Add persistent graph definitions, versioning, parameters, and reusable templates. |
| Worker model | Workers poll dedicated task queues and report status. | Tasks run inside stage execution with retry and remediation behavior. | Worker manifests declare functions, resources, triggers, and sandbox policy; `OpenViewControlApi` polls compatible queue tasks and requires current leases for complete/fail; process supervision emits sanitized heartbeat call-site events for lease extension. | Add full process-backed task execution, task output persistence, and graph advancement. |
| Queues | Each task type has a queue. Work can be distributed and scaled. | Execution queue drives stage/task progress. | `QueueTask` and `OpenViewControlApi` cover enqueue, poll, lease, visibility timeout, current-lease complete/fail, retry, dead-letter, stale lease rejection, and expired-lease recovery. | Bind to iii-queue/database tables, own production migrations, and enforce or surface per-worker concurrency policy. |
| State history | Workflow and task metadata are persisted and queryable by execution id. | Execution history doubles as audit log for deployment processes. | `LocalRunStore` persists workflows, runs, events, queue tasks, approvals, worker runtimes, artifacts, restart snapshots, and evidence bundle summaries. | Add production database store, execution index/search, full event-sourced replay, and CLI/UI history views. |
| Retries and timeouts | Tasks can be retried, failed, terminated, or timed out. | Retryable tasks can define backoff and timeout behavior. | Queue failures can retry with backoff or enter terminal failed/dead-letter state; runtime supervision tracks worker failure/restart. | Add timeout integration with live workers and production adapter retry policy configuration. |
| Human approval | Wait tasks and external signals can pause progress. | Manual judgment gates deployments. | Approval gate blocks and resumes a run. | Add approval queue listing, expiration, approver identity, resolution comments, and notification hooks. |
| Sandbox and policy | Worker execution is isolated by worker deployment and task type. | Deployment actions are constrained by account/provider permissions. | Sandbox profile and policy checks are first-class. | Add actual local enforcement for commands, paths, network, credentials, and process groups. |
| Observability | APIs expose workflow status and metadata. Search/index backends support lookup. | UI shows pipeline execution history and stage/task details. | Docs describe Agent View panes and trace timeline. | Build CLI watch, event streaming API, trace timeline data model, and dashboard endpoints. |
| Extensibility | New task types are implemented by workers and registered with definitions. | New stages require backend stage logic and task graph wiring. | Worker catalog plus function/trigger registration is in place. | Add manifest validation, package install flow, compatibility checks, and worker SDK examples. |
| Persistence | Pluggable stores hold workflow metadata, task queues, and execution history. | SQL/Redis-backed queues and execution repositories are common. | `LocalRunStore` covers the local contract surface; live iii E2E can persist task lease, heartbeat, recovery, retry, and dead-letter state to `$III_E2E_DEMO_DIR/openview-e2e.db` by default. | Add production database-backed store, owned table definitions, migrations, and query/index tests. |
| Parallel execution | Workflows can fan out tasks and join results. | Pipelines support parallel stage paths. | Agent graph edges and shared resources exist. | Add fanout/fanin nodes, join semantics, race cancellation, and parallel evidence aggregation. |
| API boundary | REST/gRPC APIs start workflows, poll tasks, update status, and inspect execution. | Gate/UI APIs expose pipeline operations and history. | Backend-compatible messages and an in-process `OpenViewControlApi` cover register workflow, enqueue, poll, complete, fail, approve, reject, cancel, and read events. | Expose the same operations through production adapter/HTTP/CLI surfaces. |

## Missing features by layer

### 1. Workflow and graph layer

Missing:

- Persisted workflow definitions with ids, versions, owners, and schemas.
- Parameter binding for graph inputs.
- Fanout and fanin nodes.
- Conditional branches.
- Cancellation semantics.
- Compensation hooks for rollback or cleanup.
- Template library for common agent workflows: repo hardening, issue triage, release review, incident response, docs update.

Acceptance tests:

- Given a graph with planner, two parallel builders, reviewer, and approval gate, `compile()` emits a deterministic plan with fanout, join, and required workers.
- Given a graph version and input payload, OpenView validates the payload before enqueueing the run.
- Given a cancelled run, downstream tasks are not scheduled and a `run.cancelled` event is persisted.
- Given a failed task with a compensation hook, the hook is scheduled once and its output is attached to the run evidence.

### 2. Queue and worker execution layer

Implemented contract surface:

- Durable queue task state with queue name, payload, retry count, visibility timestamp, lease owner, completed timestamp, last error, and stable queue event sequence.
- Worker polling through `OpenViewControlApi` that leases only visible tasks compatible with the polling worker id.
- Lease ownership and visibility timeout, including stale lease rejection and expired-lease reclamation.
- Lease heartbeat extension and bounded polling for local per-queue concurrency limits.
- Complete/fail/requeue operations that require the current lease.
- Retry backoff and terminal failed/dead-letter state after retry exhaustion.
- Database-backed task-state primitive check for lease, heartbeat, recovery, retry, and dead-letter state in live iii E2E when `OPENVIEW_III_E2E_DATABASE=1` is enabled.
- Deterministic CLI readiness output for queue schema records, migration surface, worker heartbeat-to-lease status, and live database E2E lifecycle.

Still missing:

- Production binding to iii-queue named queues or owned iii-database lease tables.
- Worker heartbeat tied to active leases in live process workers.
- Production iii-queue concurrency policy and metrics surfaced through observable adapter state.
- Task output persistence and graph advancement from completed/failed tasks.

Acceptance tests:

- Passing now: a worker polling `shell.sandbox` only receives tasks for `shell.sandbox` functions.
- Passing now: stale complete/fail requests are rejected before mutating task state.
- Passing now: a leased task becomes visible again after visibility timeout and can be reclaimed.
- Passing now: a current lease can heartbeat to extend its visibility deadline.
- Passing now: bounded polling prevents over-leasing a queue beyond its local concurrency limit.
- Passing now: failing a task with retries remaining schedules the next attempt after backoff.
- Passing now: failing a task with no retries remaining moves the task to terminal failed/dead-letter state.
- Still needed: completing a task stores output, advances the run, and emits `task.completed` and `run.advanced` events in the production runner.
- Passing now: process-supervisor heartbeat call-site events point at `OpenViewControlApi::heartbeat_worker_leases`, and lease loss/recovery behavior is covered by queue visibility-timeout contracts.

### 3. Runtime supervision layer

Missing:

- Actual child-process launcher.
- Stdout/stderr log capture.
- Resource cleanup on stop.
- Restart policy with backoff and max restarts.
- Readiness checks from worker manifest.
- Health endpoint or stdio health handshake.

Acceptance tests:

- Starting a worker runtime records pid, command, env key names, working directory, and `runtime.started`.
- A ready signal moves the worker from `starting` to `ready`.
- A crashed worker records exit code, failure reason, and captured logs without exposing secret values.
- A restart policy restarts the worker until max restarts, then moves it to `failed`.
- Stopping a worker kills the process group and emits `runtime.stopped`.

### 4. Sandbox enforcement layer

Missing:

- Local command runner that enforces the declared sandbox profile.
- Canonical path enforcement against symlink/path traversal escapes.
- Network allowlist enforcement.
- Credential injection by name only.
- Per-function approval before risky execution.
- Redaction for logs and events.

Acceptance tests:

- A command not listed in `allowed_commands` is rejected before spawn.
- A write outside `write_roots` is rejected before the command receives access.
- A denied root overrides a broader allowed root.
- A network request to an unlisted host fails under sandbox policy.
- A credential value never appears in events, logs, task output, or worker runtime metadata.

### 5. Approval and human-in-the-loop layer

Missing:

- Approval queue API.
- Approver identity and role checks.
- Approval expiration.
- Resolution comments.
- Notification hooks.
- Rejection path that can fail, reroute, or request a safer plan.

Acceptance tests:

- A risky function creates an approval record with function id, worker id, risk, reason, run id, and requested payload summary.
- Approving resumes the blocked run and records approver id plus comment.
- Rejecting records resolution and moves the run to `failed` or `needs_revision` based on policy.
- Expired approvals emit `approval.expired` and stop scheduling downstream tasks.
- The approval API never returns raw credential values or full secret-bearing payloads.

### 6. Persistence and replay layer

Implemented contract surface:

- Local store for workflow definitions, run snapshots, run events, approvals embedded in runs, queue tasks, worker runtimes, and artifacts.
- Event sequence guarantees and gap rejection before replay/evidence export.
- Restart snapshot recovery for phase, approvals, events, current queue status, active leases, and artifacts.
- Evidence bundle export with events, approvals, queue task summaries, worker runtime metadata, and artifact metadata without payload bytes.
- Optional live iii database task-state path at `$III_E2E_DEMO_DIR/openview-e2e.db`, defaulting to `/private/tmp/openview-iii-e2e/openview-e2e.db`.

Still missing:

- Production database-backed store with owned table definitions and migrations.
- Full event-sourced replay from append-only events rather than snapshot plus events.
- Search/index surface for execution history and time-range queries.
- Production store query coverage for leases, failures, restart counts, and health.

Acceptance tests:

- Passing now: a run can be restarted from persisted snapshot state after process restart.
- Passing now: event sequences are monotonic per run and gapful snapshots are rejected.
- Passing now: evidence export includes queue tasks, approvals, worker runtime metadata, events, and artifacts.
- Still needed: replaying append-only events reconstructs the same run phase, task attempts, approvals, outputs, and failures.
- Still needed: querying by run id returns graph, phase, current tasks, approvals, event history, and artifact links from the production store.
- Still needed: querying by worker id returns current leases, recent failures, restart count, and runtime health from the production store.

### 7. API and adapter layer

Missing:

- Local HTTP API or adapter trait for runtime operations.
- Start workflow endpoint.
- Poll/lease task endpoint.
- Complete/fail task endpoint.
- Approve/reject endpoint.
- Cancel endpoint.
- Event stream endpoint.
- Backend adapter implementation behind traits.

Acceptance tests:

- `POST /runs` validates inputs, creates a run, schedules the first tasks, and returns a run id.
- `POST /workers/{id}/poll` leases only compatible tasks.
- `POST /tasks/{id}/complete` validates output schema and advances the run.
- `POST /approvals/{id}/approve` resumes a blocked run.
- `GET /runs/{id}/events` streams ordered events without skipping sequence numbers.

### 8. Control plane and UX layer

Missing:

- CLI watch mode.
- Worker catalog inspection from persisted state.
- Approval queue listing/filtering.
- Trace timeline data model.
- Workspace session persistence.
- Run evidence bundle export.
- Dashboard endpoints for Agent View.

Acceptance tests:

- `openview runs watch <run_id>` prints state transitions in event order.
- `openview approvals list` shows pending approvals without secret payloads.
- `openview workers status` shows health, restart count, last heartbeat, and current leases.
- `openview queues readiness` shows queue schema/migration status, heartbeat-to-lease call-site status, concurrency policy observability, and live database E2E lifecycle without secret values.
- `openview evidence export <run_id>` writes logs, events, outputs, approvals, and artifacts into a reviewable bundle.
- Restoring a workspace session reopens the same run, trace, terminal, worktree, and approval panes.

## Milestones

### Milestone 0: Contract foundation lock

Goal: make the current Rust foundation clean, documented, and protected.

Deliverables:

- Keep worker manifest, graph compile, run approval, runtime supervision, sandbox policy, and backend message tests green.
- Add this roadmap to `docs/plans/`.
- Keep public-copy guard passing.

Acceptance:

```bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/check_public_copy.sh
```

### Milestone 1: Durable run store

Goal: make runs survive process restart.

Status: local store/recovery contracts pass; production database store remains open.

Deliverables:

- Done: local records for runs, tasks, events, approvals embedded in runs, worker runtimes, workflow definitions, and artifacts.
- Done: event sequence model with monotonic gap rejection.
- Done: snapshot restart validation for run phase, approvals, events, queue lease state, and artifacts.
- Remaining: production database-backed store, owned migrations, and event-sourced reconstruction from append-only events.

Acceptance:

- Passing now: start a run, persist it, restart from a snapshot, reload by run id, and preserve phase plus pending approvals.
- Passing now: event order is deterministic and gap-free per run.
- Still needed: production replay reconstructs state from append-only events and continues queued tasks through live workers.

### Milestone 2: Queue and lease protocol

Goal: move from in-memory plans to queue-backed worker execution.

Status: queue protocol contracts pass; production iii queue/database binding and live worker ownership remain open.

Deliverables:

- Done: task queue state and in-process control API for enqueue, poll, lease, complete, fail, and read events.
- Done: visibility timeout, heartbeat extension, stale lease rejection, expired lease recovery, retry policy, requeue, local concurrency limit, and terminal failed/dead-letter state.
- Done: live iii E2E database primitive check for task lease, heartbeat, recovery, retry, and dead-letter state.
- Done: deterministic CLI readiness shim for queue schema/migration state, worker heartbeat-to-lease integration status, and live database E2E lifecycle.
- Remaining: production iii-queue/database implementation, task output/graph advancement, full runner/API wiring, and UI exposure.

Acceptance:

- Passing now: a worker can poll compatible tasks and complete/fail them only with the current lease.
- Passing now: expired leases become eligible for another worker by visibility timeout.
- Passing now: retry attempts are counted and delayed by policy.
- Passing now: exhausted retries land in terminal failed/dead-letter state with failure evidence.
- Still needed: completed tasks advance a production run and live worker heartbeats extend or release leases according to policy.

### Milestone 3: Process-backed workers

Goal: supervise real worker binaries instead of only modeling runtime specs.

Deliverables:

- Child-process launcher.
- Log capture.
- Health/readiness handshake.
- Restart policy.
- Graceful stop and process group cleanup.
- Worker manifest validation before launch.

Acceptance:

- A sample worker starts, reports ready, receives a leased task, completes it, and exits cleanly.
- A crashed worker is restarted according to policy and emits failure evidence.
- Secret values are not stored in runtime metadata or logs.

### Milestone 4: Enforced sandbox runner

Goal: make policy real for local commands.

Deliverables:

- Command allowlist enforcement.
- Filesystem read/write root enforcement.
- Denied root override.
- Network host allowlist enforcement.
- Credential-name-only injection.
- Redaction and event hygiene.

Acceptance:

- Attempts to run unapproved commands, write outside roots, access denied paths, call disallowed hosts, or print secret values are blocked or redacted with explicit events.
- Approval-required functions cannot execute until the approval record is resolved.

### Milestone 5: Approval queue and review loop

Goal: make human decisions first-class and inspectable.

Deliverables:

- Approval queue API.
- CLI approval list/show/approve/reject.
- Approval expiration.
- Reviewer comments.
- Rejection routing to fail, revise, or safer-plan states.

Acceptance:

- A blocked run appears in `openview approvals list`.
- Approving resumes the run and records approver identity plus comment.
- Rejecting stops or reroutes the run according to policy.
- Every approval can be exported in the run evidence bundle.

### Milestone 6: Parallel agent operating model

Goal: support Hermes-style parallel workers as a normal OpenView workflow.

Deliverables:

- Fanout/fanin graph nodes.
- Worktree-scoped builder workers.
- Reviewer worker that depends on builder outputs.
- Evidence aggregation across child runs.
- Parent run summary with child status and artifacts.

Acceptance:

- A parent run creates planner, builder, reviewer, and writer branches with separate worktree bindings.
- Builders can run in parallel without writing outside their assigned workspace roots.
- Reviewer does not start until required builders complete or fail according to policy.
- Parent run collects child artifacts, test logs, approval decisions, and review comments into one evidence bundle.

### Milestone 7: Control plane APIs and Agent View

Goal: make OpenView visible enough for operators.

Deliverables:

- Run history API.
- Event stream API.
- Worker status API.
- Approval API.
- Workspace session persistence.
- Trace timeline and evidence export models.

Acceptance:

- A UI or CLI can render running, blocked, failed, cancelled, and completed runs from APIs only.
- Trace timeline shows task attempts, worker leases, approvals, policy decisions, logs, and artifacts in order.
- Workspace restore opens the same runs, panes, worktrees, terminals, and approval queue.

## Hermes parallel-agent operating model

OpenView should treat the Hermes board pattern as a reference operating model for serious agent work.

### Roles

- Planner: decomposes a goal into scoped tasks with dependencies.
- Builder: makes code or docs changes inside a bounded workspace.
- Reviewer: inspects diffs, tests, public-copy rules, and safety constraints.
- Writer: turns technical state into clear docs, plans, launch copy, or handoff notes.
- Operator: approves risky actions, resolves ambiguity, and owns final merge/release decisions.

### Rules OpenView should encode

- One worker owns one bounded task.
- Workspaces are explicit resources, not implicit filesystem access.
- Parent-child dependencies decide when work becomes eligible.
- Review-required work blocks until a human or reviewer resolves it.
- Every run writes structured handoff metadata.
- Follow-up work becomes a new task, not hidden scope creep.
- Long operations heartbeat with progress.
- Public artifacts pass copy and secret guards before completion.

### OpenView workflow shape

```text
parent run
  ├─ planner
  │   └─ emits task graph and dependency edges
  ├─ builder A
  │   └─ owns worktree A, emits diff + tests
  ├─ builder B
  │   └─ owns worktree B, emits diff + tests
  ├─ writer
  │   └─ owns docs/copy artifacts
  └─ reviewer
      └─ waits for builders/writer, verifies evidence, blocks or approves
```

### Acceptance tests for the operating model

- A parent run with three child tasks does not mark complete until all required children complete or a policy-defined failure path is chosen.
- A child worker cannot write outside its assigned workspace root.
- A reviewer can read child handoffs without re-running child work.
- A review-required block prevents merge/release until resolved.
- A parent evidence bundle includes child summaries, metadata, logs, approvals, and artifacts.

## Definition of parity

OpenView reaches practical parity when an operator can run this flow without bespoke glue:

1. Register worker manifests for planner, builder, reviewer, writer, shell sandbox, git worktree, approval gate, storage, and event stream.
2. Start a durable graph with typed inputs.
3. Queue tasks by worker type.
4. Lease work to compatible workers.
5. Enforce sandbox and approval policy before side effects.
6. Persist all events, approvals, outputs, logs, and artifacts.
7. Retry safe failures and dead-letter unsafe failures.
8. Show execution history through CLI/API/control plane.
9. Export an evidence bundle for human review.
10. Resume or replay the run after process restart.

At that point OpenView is no longer only a contract library. It becomes a small, auditable control plane for agent work.

## Remaining non-code gaps

- No separate documentation-only parity blockers are tracked after this update; remaining blockers are production runtime/API/UX implementation and live verification.
- Public roadmap language should continue to distinguish passing queue/store contract surfaces from full production workflow parity.

## Source notes

- Conductor architecture docs describe a worker-task queue architecture, task queues per task type, polling workers over HTTP/gRPC, server-managed workflow state, retries/failures, and persistent workflow/task metadata.
- Conductor worker docs describe writing workers, scaling workers, and task routing across worker groups.
- Spinnaker pipeline docs describe pipelines as ordered and parallel stages, stage/task execution, manual judgment, execution history, and audit logs for deployment operations.
- Spinnaker stage-extension docs describe backend stages as task graphs with cancellable/restartable/retryable behavior and explicit task classes.
