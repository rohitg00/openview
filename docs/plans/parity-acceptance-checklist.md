# Parity Acceptance Checklist

## Purpose

This checklist defines what OpenView must prove before claiming full parity with an agent IDE control plane and durable workflow references. The current implementation direction is iii primitive parity: OpenView keeps the Rust control-plane contracts and maps durable runtime mechanics onto iii queue, database, stream, approval, session, shell, and harness workers where available.

Parity means an operator can start a typed agent workflow, fan work out to bounded workers, pause for approval, enforce sandbox policy, persist every event, recover after restart, inspect the run through CLI/API/UI surfaces, and export review evidence without bespoke glue.

This document intentionally uses OpenView terms, iii primitive names, and public system categories only. Do not add restricted exact reference names that fail `scripts/check_public_copy.sh`.

See `docs/plans/iii-adapter-parity.md` for the detailed mapping from Absurd-style tasks, runs, checkpoints, events, waits, idempotency, retry, and cancellation into iii primitives.

## Current status snapshot

Last verified from `/Users/rohitsagent/openview`:

| Area | Status | Evidence |
|---|---:|---|
| Rust contract tests | PASS | `cargo test -p openview-cli` passed: 10 tests, 0 failures for the operator shim tranche. Full workspace gates are tracked separately. |
| Public-copy guard | PASS | `scripts/check_public_copy.sh` returned `Public copy check passed`. |
| Worker manifest contracts | PASS | Built-in worker catalog, worker crate manifests, resources, sandbox profile, and safe env key-name handling are covered by tests. |
| Graph and run-state contracts | PASS | Planner/build/review-style graph, approval blocking, and resume-to-complete state are covered by tests. |
| Event stream contracts | PASS | Cursor, pagination, empty reads, unknown run errors, and approval-flow replay are covered by tests. |
| Queue lease/retry contracts | PASS | `QueueTask` and `OpenViewControlApi` cover enqueue, compatible polling, lease ownership, visibility timeout, stale lease rejection, complete/fail, retry backoff, dead-letter terminal state, explicit requeue, and expired-lease recovery. |
| Durable store/recovery contracts | PASS | `LocalRunStore` covers workflow definitions, runs, events, approvals, worker runtimes, queue tasks, artifacts, monotonic event sequences, restart snapshots, current queue lease state, and deterministic evidence bundles. |
| Review workflow contract | PASS | Merge-readiness rules, blocking rules, and deterministic serialization are covered by tests. |
| Sandbox policy contract | PASS | Filesystem, denied roots, command identity, network host, and secret-name policy are covered by tests. |
| iii primitive adapter parity | PARTIAL | Contract tests cover the adapter plan, OpenView owns the queue lease/retry/recovery contract surface, `queues readiness` exposes deterministic schema/migration, heartbeat-to-lease call-site, concurrency policy, and live database E2E lifecycle state, and `scripts/e2e_iii_runtime.sh` exercises a live iii engine plus harness workers across model catalog, streams, session-tree, approvals, durable publish, hook fanout, REST bridge, shell execution, owned queue schema bootstrap, and optional `iii-database` transaction/query/execute task-state persistence. Production iii-queue delivery, full process execution, graph advancement, and UI/API wiring are still required. |
| Agent IDE parity | PARTIAL | Workspace session and pane models exist, and deterministic CLI shims expose demo approvals, run events, worker status, queue adapter readiness, and evidence export. Durable live APIs and UI panes are still missing. |
| Durable workflow reference parity | PARTIAL | Queue, store, replay, retry, control API, evidence, and worker-supervision contract surfaces now pass. Full parity still requires production-backed queue/store implementations, process execution, sandbox enforcement, live UI/API surfaces, and end-to-end restart/retry/cancellation drills. |

## Tests that currently pass

### Core contract tests

- `builds_agent_graph_that_connects_planner_approval_sandbox_and_model_workers`
- `openview_runtime_supervises_worker_start_heartbeat_fail_restart_and_stop`
- `worker_runtime_spec_tracks_binary_lifecycle_and_safe_env_key_names_only`
- `registers_rust_workers_with_capabilities_resources_and_sandbox_policy`
- `workspace_session_models_control_plane_tabs_panes_and_worktrees`
- `emits_three_i_compatible_registration_messages_without_engine_dependency`
- `run_state_machine_blocks_on_approval_then_resumes_to_completed`
- `git_worktree_manifest_declares_locked_down_contract`
- `terminal_pty_manifest_declares_locked_down_contract`
- `built_in_catalog_contains_first_class_git_worktree_worker`
- `built_in_catalog_contains_first_class_terminal_pty_worker`

### Event stream tests

- `event_stream_unknown_run_errors`
- `event_stream_empty_reads_keep_stable_cursor`
- `approval_flow_events_are_replayable_in_sequence_by_cursor`
- `event_stream_pagination_respects_limit_and_has_more`

### Review workflow tests

- `review_workflow_is_merge_ready_when_comments_checks_and_todos_are_resolved`
- `review_workflow_blocks_merge_on_unresolved_comments_failing_checks_and_open_todos`
- `review_workflow_serializes_as_deterministic_snake_case_contract`

### Sandbox policy tests

- `network_policy_requires_allowed_host_when_network_is_enabled`
- `filesystem_deny_roots_override_less_specific_allow_roots`
- `filesystem_policy_allows_only_declared_read_and_write_roots`
- `process_policy_requires_exact_allowed_command_identity`
- `secrets_are_referenced_by_allowed_name_only`

### Queue and store tests

- `queue_task_starts_available_with_payload_and_no_retry_debt`
- `lease_claim_hides_task_until_visibility_timeout_and_emits_deterministic_event`
- `ack_complete_requires_current_lease_and_terminally_completes_task`
- `fail_with_retry_budget_requeues_with_incremented_retry_count_and_visibility_delay`
- `fail_without_retry_budget_dead_letters_task_without_requeueing`
- `explicit_requeue_requires_current_lease_and_preserves_retry_count`
- `expired_lease_can_be_reclaimed_without_threads_or_runtime`
- `queue_contract_serializes_with_snake_case_status_and_stable_events`
- `local_run_store_persists_runs_events_definitions_and_queries_by_runtime_state`
- `persist_event_rejects_non_monotonic_or_gapful_run_sequences`
- `restart_snapshot_restores_phase_approvals_events_and_current_queue_status`
- `restart_snapshot_rejects_gapful_persisted_event_streams_before_replay`
- `typed_control_api_registers_workflows_and_reads_ordered_event_pages`
- `worker_polling_only_leases_visible_tasks_compatible_with_that_worker`
- `completing_and_failing_tasks_require_current_lease_and_preserve_dead_letter_semantics`
- `evidence_bundle_export_is_deterministic_and_contains_only_contract_metadata`
- `snapshot_restart_preserves_artifacts_for_bundle_export`

### Worker crate tests

- `worker_crate_exposes_git_worktree_manifest_for_cli_use`
- `worker_crate_exposes_terminal_pty_manifest_for_cli_use`
- `worker_crate_exposes_hermes_agent_manifest_contract`

### CLI shim tests

- `operator_parity_commands_are_exposed_as_top_level_shims`
- `queue_readiness_reports_schema_heartbeat_and_live_e2e_state_without_secrets`

### Live iii runtime E2E

- `scripts/e2e_iii_runtime.sh` validates a running iii stack with real function calls:
  `harness::status`, `models::list`, `stream::set/list`, `session-tree::create/append/messages`,
  `approval::list_pending`, `iii::durable::publish`, `engine::queue::list_topics`,
  `hook-fanout::publish_collect`, REST `bridge/trigger`, and `shell::exec`.
- `OPENVIEW_III_E2E_DATABASE=1 OPENVIEW_III_MANAGED=1 scripts/e2e_iii_runtime.sh`
  also starts `iii-database` and validates transaction, query, execute, and replay-style read paths
  for durable task lease, heartbeat, recovery, retry, and dead-letter state. The default live SQLite task-state file is
  `$III_E2E_DEMO_DIR/openview-e2e.db`, which resolves to
  `/private/tmp/openview-iii-e2e/openview-e2e.db` unless overridden.
- `OPENVIEW_RUN_LIVE_III_E2E=1 cargo test -p openview-worker --test live_iii_e2e -- --nocapture`
  runs the same script from cargo when a live iii stack is already reachable.
- `OPENVIEW_III_MANAGED=1 scripts/e2e_iii_runtime.sh` starts and stops the local demo iii stack
  when the iii engine and worker binaries are available on the host.

## Acceptance checklist by layer

### 1. Workflow graph and templates

Current status: PARTIAL

Already proven:

- Worker manifests can describe functions, triggers, resources, deployment targets, sandbox policy, and security posture.
- The agent graph contract can connect planner, builder, reviewer, approval, sandbox, and model-routing roles.
- Run state can block on approval and resume to completed.

Required for parity:

- Persist workflow definitions with ids, versions, owners, schemas, and parameter bindings.
- Add reusable workflow templates for code review, issue triage, release review, docs update, incident response, and sandboxed command execution.
- Add fanout/fanin nodes, conditional branches, cancellation, and compensation hooks.
- Validate typed inputs before enqueueing a run.

Ship tests:

- Given a versioned workflow and valid input payload, OpenView stores the run and schedules the first tasks.
- Given invalid input, OpenView rejects the run before any worker receives work.
- Given planner plus two parallel builders plus reviewer, graph compilation emits deterministic fanout, join, dependencies, and worker requirements.
- Given cancellation, downstream unscheduled tasks stay unscheduled and a `run.cancelled` event is persisted.
- Given a failed task with cleanup enabled, the compensation hook is scheduled once and attached to run evidence.

### 2. Durable queue and lease protocol

Current status: PASS for OpenView queue/control contracts; PARTIAL for production iii queue integration

Already proven:

- Durable queued execution exists as a plan-level concept and as a tested local queue/control API surface.
- Worker capabilities and function ids are available in manifests.
- `QueueTask` owns task state, queue name, payload, retry count, visibility timestamp, lease owner, completed timestamp, last error, and stable queue event sequence.
- `OpenViewControlApi` owns enqueue, compatible worker poll, lease claim, current-lease completion, current-lease failure, stale lease rejection, and event-page reads.
- Retry semantics are deterministic: retryable failures increment retry count, delay the next visible time, and emit failed/requeued events; exhausted retries enter terminal failed state with failure evidence.
- Recovery semantics are deterministic: expired leases become visible again and can be reclaimed without requiring threads or a live runtime.
- Worker heartbeat is modeled in runtime supervision contracts, queue lease heartbeat is modeled through `QueueTask`, and queue recovery is modeled through visibility timeout/expired-lease release.
- Bounded polling can enforce local per-queue concurrency limits before another lease is granted.
- The live iii E2E can persist and update task lease, heartbeat, recovery, retry, and dead-letter state through `iii-database::transaction`,
  `iii-database::query`, and `iii-database::execute` against SQLite.
- The CLI exposes `queues readiness` and `control-plane queue-readiness` as deterministic operator state for schema/migration ownership, worker heartbeat-to-lease call-site wiring, concurrency policy observability, and the live database E2E lifecycle.

Required for parity:

- Bind the queue/control API to `iii-queue` named queues when available and to `iii-database` lease tables when queue support is unavailable or insufficient.
- Tie process-worker heartbeats to active task leases, including lease extension or expiry behavior in live workers.
- Surface delegated iii-queue concurrency policy and production concurrency metrics as explicit OpenView adapter state.
- Persist task outputs and failure evidence.
- Use owned `iii-database` table definitions and transitions for task state, idempotency keys, leases, retry scheduling, and dead-letter records in the production fallback.
- Drive graph advancement from completed/failed queue tasks in an end-to-end workflow runner.

Ship tests:

- Passing now: a worker polling `shell.sandbox` only receives compatible `shell.sandbox` tasks.
- Passing now: stale completion/failure leases are rejected before mutating task state.
- Passing now: a leased task becomes visible again after visibility timeout and can be reclaimed.
- Passing now: failing a retryable task schedules the next attempt after backoff.
- Passing now: exhausting retries moves the task to terminal failed/dead-letter state.
- Still needed: completing a task stores output, advances the graph, and emits `task.completed` plus `run.advanced` in the production runner.
- Passing now: process supervisor heartbeat events name the `OpenViewControlApi::heartbeat_worker_leases` call-site with sanitized metadata, and missed heartbeats recover by visibility timeout in the queue contract.
- Passing now: production iii-queue concurrency policy can be represented as delegated or locally enforced observable adapter state through `QueueConcurrencySnapshotResponse`.

### 3. Durable store and replay

Current status: PASS for local store/recovery contracts; PARTIAL for production database-backed store

Already proven:

- Event-stream contract tests define cursor and pagination behavior.
- Approval-flow events can be replayed by cursor in contract tests.
- `LocalRunStore` persists workflow definitions, run snapshots, run events, queue tasks, approvals embedded in runs, worker runtimes, and artifacts.
- Event sequences are monotonic and gapful snapshots are rejected before replay or evidence export.
- Restart snapshots preserve run phase, pending approvals, event history, current queue status, active lease details, and artifacts.
- Evidence bundles include run state, events, approvals, related worker runtime metadata, queue task summaries, and artifact metadata without embedding queue payloads or artifact bytes.
- The live iii E2E can write task state through an `iii-database` transaction, update retry state,
  and read the updated row back from the durable SQLite file at `$III_E2E_DEMO_DIR/openview-e2e.db`
  by default.

Required for parity:

- Add a production database-backed store abstraction over SQLite/iii-database, with owned table definitions and migrations.
- Guarantee monotonic event sequence per run, mirrored to `stream::set` for live control-plane updates and read through `stream::list` when stream storage is the active event surface.
- Reconstruct run state from persisted events, not only from a stored run snapshot plus event history.
- Query runs by run id, worker id, approval state, and time range.

Ship tests:

- Passing now: a run survives snapshot restart with the same phase, approvals, events, current queue state, and active lease.
- Passing now: event sequence numbers are gap-free per run and gapful snapshots are rejected.
- Passing now: querying by worker id returns matching runs, and worker runtime state can be persisted and restored.
- Passing now: evidence export includes queue task summaries, runtime metadata, approvals, events, and artifacts.
- Still needed: event replay reconstructs phase, task attempts, approvals, outputs, and failures from the append-only event log alone.
- Still needed: querying by run id/time range returns graph, phase, current tasks, approvals, event history, artifact links, leases, recent failures, restart count, and health from the production store.

### 4. Process-backed workers and supervision

Current status: PARTIAL

Already proven:

- Runtime specs model binary, args, env key names, working directory, health, heartbeat, restart count, failure reason, and lifecycle events.
- Supervision events cover start, heartbeat, failure, restart, and stop in tests.

Required for parity:

- Launch real child processes.
- Capture stdout/stderr with redaction.
- Implement readiness checks from manifest metadata.
- Apply restart policy with backoff and max restarts.
- Stop process groups and clean resources.
- Tie worker heartbeat to leased tasks.

Ship tests:

- Starting a sample worker records pid, command, env key names, working directory, and `runtime.started`.
- A ready signal changes worker health from `starting` to `ready`.
- A crashed worker records exit code, sanitized logs, and failure reason.
- Restart policy restarts until max restarts, then marks the worker `failed`.
- Stopping a worker terminates the process group and emits `runtime.stopped`.

### 5. Sandbox and permission enforcement

Current status: PARTIAL

Already proven:

- Contract tests cover filesystem roots, denied roots, exact command identity, network host allowlist, and secret-name references.

Required for parity:

- Implement the local runner that enforces policy before spawn.
- Canonicalize paths to block symlink and traversal escapes.
- Enforce network host allowlists.
- Inject credentials by configured name only.
- Redact secret values from logs, events, task output, and runtime metadata.
- Require approval before risky side effects.

Ship tests:

- Unapproved commands are rejected before spawn.
- Writes outside declared roots are rejected before process access.
- Denied roots override broader allowed roots.
- Disallowed network hosts fail under policy.
- Secret values never appear in logs, events, outputs, or runtime metadata.
- Approval-required functions do not execute until approval is resolved.

### 6. Human approval and review loop

Current status: PARTIAL

Already proven:

- Run state can block on approval and resume.
- Review workflow tests can block merge readiness on unresolved comments, failing checks, and open todos.

Required for parity:

- Add approval queue API and CLI commands backed by `approval::list_pending` and `approval::resolve` where the approval worker is available.
- Store approver identity, role, comment, expiration, and resolution.
- Support rejection policies: fail, request revision, or route to safer plan.
- Emit notification hooks through `hook-fanout::publish_collect` before risky function calls and before/after operator decisions.
- Export approvals in evidence bundles.

Ship tests:

- Risky function calls create approval records with function id, worker id, risk, reason, run id, and sanitized payload summary.
- `openview approvals list` shows blocked runs without secret payloads. Current shim: `openview approvals list` prints deterministic demo approval state.
- Approving resumes the run and records approver id plus comment. Current shim: `openview approvals approve-demo` resolves the deterministic demo approval and emits `approval.approved` plus `run.completed`.
- Rejecting moves the run to `failed` or `needs_revision` according to policy. Current shim: `openview approvals reject-demo` resolves the deterministic demo approval and emits `approval.rejected` plus `run.failed` without secret-like strings.
- Expired approvals emit `approval.expired` and stop downstream scheduling.

### 7. API and adapter boundary

Current status: PARTIAL

Already proven:

- Backend-compatible registration messages exist as contracts.

Required for parity:

- Add runtime API or adapter trait for start, enqueue, poll, lease, complete, fail, approve, reject, cancel, and stream.
- Keep the OpenView core behind an iii adapter trait: `iii-queue` for task delivery, `iii-database` for transactional fallback, `stream::set` / `stream::list` for event visibility, `approval::resolve` / `approval::list_pending` for human gates, `run::start_and_wait` for Hermes agent turns, `session-tree::*` for transcript/history, shell sandbox for execution, and harness for UI/event composition.
- Keep API payloads typed and schema-validated.
- Keep adapter implementation independent from worker business logic.
- Add error semantics for conflicts, invalid state transitions, stale leases, and unauthorized actions.

Ship tests:

- `POST /runs` validates input, creates a run, schedules first tasks, and returns a run id.
- `POST /workers/{id}/poll` leases only compatible tasks.
- `POST /tasks/{id}/complete` rejects stale leases and invalid output schema.
- `POST /approvals/{id}/approve` resumes a blocked run.
- `POST /runs/{id}/cancel` stops downstream scheduling.
- `GET /runs/{id}/events` streams ordered events without sequence gaps.

### 8. Agent IDE control plane

Current status: PARTIAL

Already proven:

- Workspace session models include tabs, panes, worktree bindings, restored run ids, and pane kinds.
- Pane kinds cover run view, terminal, editor, browser, notes, approval queue, worker catalog, trace timeline, and worktree.

Required for parity:

- Persist workspace sessions.
- Restore sessions across process restarts.
- Wire panes to real run, event, approval, worker, terminal, and artifact APIs.
- Add CLI watch for run transitions.
- Add evidence bundle export.
- Show worker health, restart count, current leases, and logs.

Ship tests:

- Restoring a workspace session reopens the same run, trace, terminal, worktree, worker catalog, and approval panes.
- `openview runs watch <run_id>` prints state transitions in event order. Current shim: `openview runs watch` prints the deterministic demo run events; `openview runs cancel-demo` prints a deterministic cancellation result with `run.cancelled`.
- `openview workers status` shows health, restart count, last heartbeat, current leases, and failure reason. Current shim covers demo runtime health, restart count, heartbeat, and failure reason from runtime specs.
- `openview evidence export <run_id>` writes events, logs, approvals, outputs, artifacts, and child summaries into one bundle. Current shim: `openview evidence export-demo` prints a sanitized demo JSON bundle with run events, approvals, worker runtime health, review readiness, and artifact placeholders.
- The control plane can render running, blocked, failed, cancelled, and completed runs from APIs only.

## Next Hermes parallel waves

### Wave 1: Make durability real

Status: contract and local runtime surfaces have landed; production adapter/database wiring remains.

Parallel tasks:

1. Storage worker: local run store contracts now cover runs, tasks, events, approvals, worker runtimes, workflow definitions, artifacts, restart snapshots, and evidence bundles; remaining work is a production database-backed store and migrations.
2. iii adapter worker: adapter planning covers `iii-queue`, `iii-database`, `stream::set`, `stream::list`, approvals, session-tree, shell sandbox, and harness surfaces; remaining work is live adapter implementation.
3. Queue worker: task queue, lease ownership, heartbeat extension, visibility timeout, retry, requeue, dead-letter, stale lease, expired-lease recovery, local concurrency-limit contracts, owned schema bootstrap planning and live E2E application, control API worker heartbeat-to-lease wiring, process-supervisor heartbeat call-site events, and observable concurrency policy snapshots now pass; remaining work is task output persistence, graph advancement, full production runner/API wiring, and UI exposure.
4. Replay worker: restart snapshot recovery tests now pass; remaining work is full event-sourced reconstruction from append-only events.
5. Writer worker: keep docs and public-copy guard aligned as terms change.

Wave 1 exit criteria:

- Passing now: a run persists in the local store, snapshot restart validates event sequence, and queued task lease state is preserved.
- Passing now: queue/control contracts cover compatible polling, current lease completion/failure, retry, dead-letter, stale lease rejection, and expired lease recovery.
- Still open: production restart replay reconstructs state from append-only events and queued tasks continue from the right phase through live workers.
- Tests do not require the `iii` CLI on `PATH`; live iii checks must use a configured binary path or API.
- `cargo test --workspace` and `scripts/check_public_copy.sh` pass.

### Wave 2: Execute real workers safely

Parallel tasks:

1. Runtime worker: implement child-process launch, readiness, restart, stop, and log capture.
2. Sandbox worker: implement command/path/network/credential enforcement in the local runner.
3. Approval worker: implement approval queue API, expiration, approve/reject transitions, and sanitized payload summaries.
4. Reviewer worker: verify secret redaction, policy bypass attempts, and stale lease behavior.

Wave 2 exit criteria:

- A sample worker launches, leases compatible work, completes a task, emits evidence, and exits cleanly.
- Policy blocks unapproved commands, denied paths, disallowed hosts, and unresolved approvals before side effects.

### Wave 3: Expose the operating surface

Parallel tasks:

1. API worker: implement start, poll, complete, fail, approve, reject, cancel, and event stream endpoints or adapter methods.
2. CLI worker: implement `runs watch`, `approvals list/show/approve/reject`, `workers status`, and `evidence export`.
3. Control-plane worker: persist workspace sessions and connect panes to run/event/worker/approval/artifact data.
4. Docs worker: update README, control-plane docs, and parity docs with verified commands only.

Wave 3 exit criteria:

- Operators can inspect every run state through CLI/API/control-plane surfaces.
- Evidence export is sufficient for human review without re-running child work.

### Wave 4: End-to-end parity drill

Parallel tasks:

1. Planner worker: define a realistic repo-change workflow with planner, parallel builders, reviewer, writer, approval, sandbox, and artifact export.
2. Builder workers: execute bounded workspace tasks in parallel without cross-writing resources.
3. Reviewer worker: verify diffs, tests, approvals, public-copy guard, and evidence bundle.
4. Operator worker: run restart, retry, cancellation, rejection, and dead-letter scenarios.

Wave 4 exit criteria:

- OpenView runs the full workflow without manual database edits or one-off scripts.
- Failure, retry, approval, cancellation, restart, and evidence-export paths are all demonstrated.

## Final ship criteria

OpenView can claim parity only when all of these are true:

- `cargo fmt --all` passes.
- `cargo test --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `scripts/check_public_copy.sh` passes.
- `scripts/e2e_iii_runtime.sh` passes against a live iii engine and harness worker stack.
- A durable workflow can be started from typed input.
- Tasks are persisted, queued, leased, heartbeated, completed, failed, retried, or dead-lettered with durable events.
- Worker processes are supervised with readiness, logs, restarts, stop behavior, and sanitized metadata.
- Sandbox policy is enforced before side effects.
- Human approvals are durable, inspectable, expirable, and exportable.
- Event replay reconstructs run state after restart.
- CLI/API/control-plane surfaces can show run state, worker state, approvals, traces, logs, artifacts, and evidence bundles.
- iii-backed paths and durable fallback paths produce equivalent run/event/evidence semantics.
- Parallel builder/reviewer/writer workflows can run with bounded workspaces and fanin evidence.
- Public docs and commits avoid restricted exact reference names and pass the public-copy guard.

## Remaining non-code gaps

- No separate documentation-only parity blockers are tracked after this update; the known blockers above are runtime/API/UX implementation and live verification gaps.
- Public copy should keep saying "contract/runtime surfaces pass" until production-backed queue/store paths, live worker ownership, and end-to-end drills pass.

Until these are true, the correct public claim is: OpenView has a strong Rust contract foundation and a clear parity path, not full production parity.
