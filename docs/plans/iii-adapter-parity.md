# iii Adapter Parity Plan

## Purpose

OpenView should pivot from building a standalone durability layer first to using iii primitives as the default runtime substrate, with OpenView owning the typed control-plane contracts, policy boundary, evidence model, and operator-facing UX.

The pattern reference is Absurd's durable execution model: a top-level task is broken into persisted steps/checkpoints; every attempt is a run; workers pull compatible work; events and waits suspend/resume execution; idempotency, retry, cancellation, and cleanup are explicit runtime concerns. OpenView should preserve those operator guarantees while mapping the backing mechanics to iii workers where possible.

## Current implementation snapshot

Treat live runtime claims as verified only when they are covered by `scripts/e2e_iii_runtime.sh` or an equivalent configured API/binary path. The script does not require `iii` to be on `PATH`; it defaults `III_BIN` to `/Users/rohitsagent/iii/target/debug/iii` and lets operators override it.

Implemented contract/runtime surfaces:

- `QueueTask` and `OpenViewControlApi` own the local queue contract for enqueue, compatible polling, lease ownership, visibility timeout, current-lease complete/fail, stale lease rejection, retry backoff, terminal failed/dead-letter state, explicit requeue, and expired-lease recovery.
- `LocalRunStore` owns the local store/recovery contract for workflow definitions, run snapshots, run events, queue tasks, approvals embedded in runs, worker runtimes, artifacts, monotonic event sequences, restart snapshots, active queue lease state, and evidence bundle summaries.
- Worker runtime heartbeat is represented in runtime supervision contracts; `OpenViewControlApi::heartbeat_worker_leases` extends active owned queue leases from worker heartbeat state; missed heartbeats still recover through visibility timeout and expired-lease reclamation; and the process supervisor emits a sanitized heartbeat event that names the control API call-site.
- Concurrency is represented locally through bounded queue polling, in the iii-queue delivery plan as delegated queue policy, and as observable adapter state through `QueueConcurrencySnapshotResponse`.
- `OPENVIEW_III_E2E_DATABASE=1 OPENVIEW_III_MANAGED=1 scripts/e2e_iii_runtime.sh` starts `iii-database`, applies the owned durable queue schema bootstrap, verifies durable tables/indexes/columns, and validates database-backed task lease, heartbeat, recovery, retry, and dead-letter state through transaction/query/execute. The default SQLite file is `$III_E2E_DEMO_DIR/openview-e2e.db`, resolving to `/private/tmp/openview-iii-e2e/openview-e2e.db` unless overridden.
- `cargo run -p openview-cli -- queues readiness` prints deterministic demo JSON for adapter readiness: queue schema/migration ownership, worker heartbeat-to-lease call-site status, concurrency policy observability, and the live iii database E2E lifecycle. It is a visibility shim, not a live adapter connection.

## Target mapping

| OpenView durability concept | Absurd reference shape | iii-backed target | Durable fallback |
|---|---|---|---|
| Task | Top-level logical work item with JSON params | Enqueue a typed function call through `iii-queue` when available, using a queue per function class or workflow lane | Store task row and state transition in `iii-database` transaction, then drive execution through the adapter loop |
| Run | Attempt to execute a task, sharing completed checkpoints | Represent each attempt as run metadata plus event records; call Hermes/agent workers through `run::start_and_wait` where the agent turn is the unit of execution | Persist run attempts, attempt numbers, lease owner, timestamps, and terminal state in `iii-database` |
| Checkpoint | Named step result persisted once and skipped on replay | Persist step/checkpoint records before side effects continue; use stream entries for operator-visible checkpoint events | Transactionally write checkpoint output and advance task/run state in `iii-database` |
| Events | Immutable signals emitted by tasks or external callers | Use `stream::set` for event items and `stream::list` for ordered replay in run-scoped streams | Store append-only event rows with monotonic sequence in `iii-database`, then mirror visible slices to streams |
| Waits | Sleep or await event until timeout/resolution | Model waits as pending state plus hook/approval/session entries; unblock through callbacks, approvals, or queue messages | Persist wait registrations and deadlines in `iii-database`; adapter scans/resumes expired or resolved waits |
| Idempotency | Spawn-time dedupe key maps to existing task | Use deterministic queue payload keys and adapter-level dedupe before enqueue | Unique idempotency table in `iii-database` transaction returns the existing task/run instead of creating another |
| Retry | Task-level retry with max attempts and backoff | Prefer `iii-queue` named queue retry, backoff, concurrency, FIFO, and DLQ semantics | Store retry policy and next_attempt_at; adapter reschedules from durable rows |
| Cancel | Mark task cancelled and stop at heartbeat/checkpoint | Publish cancel event, stop downstream queueing, and ask worker/runtime to observe cancel before next side effect | Transactionally mark run/task cancelled and reject stale completions or approvals |
| Human approval | Task suspends until external decision | Use `hook-fanout::publish_collect` before risky calls and `approval::resolve` / `approval::list_pending` for operator decisions | Store approval request/resolution rows and emit ordered events for replay/export |
| Transcript/history | Agent messages and branches | Use `session-tree::*` for transcript, branch, compaction, and resumable history | Store transcript artifacts and compacted snapshots in `iii-database` or object storage |
| Execution sandbox | Worker executes scoped command | Route command execution through the shell sandbox worker and OpenView policy checks | Reject before spawn when policy cannot be proven; persist denial evidence |
| UI/event streaming | Live run state and historical replay | Use harness plus `stream::set`/`stream::list` for control-plane panes and event streaming | Read from durable event rows and expose a compatible stream adapter |

## Adapter boundary

OpenView should introduce an `IiiRuntimeAdapter` boundary that is narrow enough to test without the live engine:

- `start_run(workflow_id, input, idempotency_key) -> run_id`
- `enqueue_task(run_id, function_id, payload, queue_policy) -> task_id`
- `record_checkpoint(run_id, task_id, step_id, output) -> checkpoint_id`
- `append_event(run_id, event_type, payload) -> sequence`
- `register_wait(run_id, wait_kind, deadline, correlation_key)`
- `resolve_approval(approval_id, decision, actor, comment)`
- `cancel_run(run_id, reason)`
- `list_events(run_id, cursor, limit)`
- `export_evidence(run_id)`

The Rust core should depend on this adapter trait, not on iii SDK calls directly. The first implementation can call iii primitives; the test implementation can use in-memory or SQLite-backed rows; a production fallback can use `iii-database` transactions when queue or stream workers are unavailable.

## Execution flow

1. Validate workflow input and idempotency key.
2. Create or load the run record.
3. Append `run.started` and mirror it to the run stream.
4. Enqueue compatible first tasks through `iii-queue` when configured.
5. Worker completion records checkpoint output before advancing graph state.
6. Risky calls publish hook fanout and create approvals when required.
7. Approved calls resume by queueing the next task; rejected calls route to fail, revise, or safer-plan policy.
8. Agent turns use `run::start_and_wait` and write transcript/history through `session-tree::*`.
9. CLI/API/UI read ordered events through `stream::list` or the database fallback.
10. Cancellation marks the run terminal and prevents stale leases, completions, approvals, or downstream task scheduling.

## Acceptance gates

OpenView can claim iii primitive parity only when all of these are true:

- A run can start from typed input and an idempotency key.
- Passing as local contracts: tasks can be enqueued, leased, completed, failed, retried, dead-lettered, requeued, and recovered after lease expiry through `QueueTask` and `OpenViewControlApi`.
- Passing as local contracts: run events are ordered, gap-free, paged by cursor, and validated during store snapshot restart.
- Passing as local contracts: task state, active lease state, worker runtime state, approvals, artifacts, and evidence summaries survive `LocalRunStore` snapshot restart.
- Passing as live primitive check: owned durable queue schema bootstrap plus database-backed task lease, heartbeat, recovery, retry, and dead-letter state can be written, updated, and replay-read from the iii E2E SQLite path when `OPENVIEW_III_E2E_DATABASE=1` is enabled.
- Passing as CLI visibility: queue adapter readiness, heartbeat call-site status, and concurrency policy observability are exposed through deterministic `queues readiness` and `control-plane queue-readiness` JSON without credentials or external services.
- Still required for production parity: tasks are durably enqueued, leased or invoked, completed, failed, retried, dead-lettered, or cancelled through the live iii adapter path.
- Still required for production parity: checkpoints are persisted before graph advancement and replay skips completed work.
- Still required for production parity: run events are ordered and replayable through `stream::list` or the database fallback in the live adapter.
- Approval requests can be listed, resolved, rejected, expired, and exported as evidence.
- Agent turn transcripts are stored through `session-tree::*` or an equivalent durable fallback.
- Shell execution is policy-checked before side effects and produces sanitized output events.
- The control plane can stream live updates and reconstruct the same state after process restart.
- Tests cover the adapter trait with no live iii dependency, plus at least one integration path against a local iii runtime when the CLI or API is available.

## Non-goals for this tranche

- Replacing iii queue, stream, approval, session, shell, or harness workers.
- Claiming production parity before live queue/database adapter execution, task output/graph advancement, cancellation, approval, and evidence flows are demonstrated end to end.
- Depending on the `iii` CLI being present on `PATH` for OpenView tests.

## Remaining non-code gaps

- No separate documentation-only parity blockers are tracked after this update; remaining blockers are live adapter implementation, process/runtime integration, API/UX exposure, and end-to-end verification.
- Public claims should keep distinguishing local contract/runtime surfaces from production iii parity until the live adapter path passes the same lease/retry/recovery scenarios.
