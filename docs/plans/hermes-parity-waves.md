# Hermes Parity Wave Tracker

## Purpose

This tracker keeps the Hermes-driven parity work concrete. It shows what is already done, what wave is active, what waves are queued next, which acceptance tests must prove each wave, and which risks need owner attention.

Target parity means OpenView can run durable agent workflows through a visible control plane: typed workflow input, queued tasks, leased workers, sandbox policy, approval gates, persisted events, restart replay, CLI/API inspection, and evidence export.

Public-copy rule: this document uses OpenView and generic public system categories only. Keep `scripts/check_public_copy.sh` passing before treating the tracker as review-ready.

## Current snapshot

Last workspace check:

```bash
cd /Users/rohitsagent/openview
pwd
git status --short
```

Observed state:

- The repository is actively being changed by multiple workers.
- This tracker owns only `docs/plans/hermes-parity-waves.md`.
- Existing uncommitted work includes Rust runtime, queue, review, sandbox, control-plane, worker, and parity-doc files owned by other workers.
- Do not use this tracker to claim implementation completion until the owning code workers have passed tests and a reviewer has accepted their handoffs.

## Completed Hermes tasks

| Task | Status | Evidence now in repo | Notes |
|---|---:|---|---|
| Contract foundation mapping | Done | existing durable-workflow parity roadmap in `docs/plans/` | Roadmap exists, but its filename/content may need public-copy cleanup by its owner before final publication. |
| Parity acceptance checklist | Done | `docs/plans/parity-acceptance-checklist.md` | Defines current pass/fail state, passing contract tests, required parity layers, next waves, and final ship criteria. |
| Worker catalog foundation | Done | `docs/workers.md` plus Rust worker manifest tests | Worker manifests, functions, resources, sandbox profile, and worker review checklist are documented. |
| Control-plane model foundation | Done | `docs/control-plane.md` plus Rust session/runtime contracts | Workspace sessions, pane kinds, runtime specs, and worker lifecycle concepts are documented. |
| Public-copy guard | Done | `scripts/check_public_copy.sh` | Guard exists and must stay green before docs are considered public-ready. |

## Active wave: Wave 1, make durability real

Goal: move OpenView from contract-shaped orchestration to restart-safe queued execution.

### Work items

| Lane | Owner type | Deliverable | Acceptance signal |
|---|---|---|---|
| Storage | Builder | Traits and local dev store for runs, tasks, events, approvals, workflow definitions, and artifacts. | A run can be reloaded after restart with graph, phase, approvals, tasks, events, and artifacts intact. |
| Queue | Builder | Task queue with poll, lease, heartbeat, visibility timeout, complete, fail, retry, and dead-letter state. | Compatible workers receive only compatible tasks; expired leases become visible; retries and dead-letter behavior are deterministic. |
| Replay | Builder | Event replay that reconstructs run state from persisted history. | Replay rebuilds phase, task attempts, approvals, outputs, failures, and current schedulable work. |
| API boundary | Builder | Trait or local API methods for start, poll, complete, fail, approve, reject, cancel, and event stream. | Invalid payloads and stale leases are rejected before mutating state. |
| Writer | Writer | Keep this tracker and parity docs aligned with real test evidence. | Public-copy guard passes and docs do not overclaim production parity. |

### Wave 1 exit criteria

- `cargo fmt --all` passes.
- `cargo test --workspace` passes with durability, queue, replay, and adapter contract tests included.
- `scripts/check_public_copy.sh` passes.
- Starting a typed run persists the run and schedules first tasks.
- A compatible worker can poll and lease a task.
- A completed task stores output, advances the run, and emits ordered events.
- A failed retryable task schedules the next attempt with backoff.
- Exhausted retries move the task to dead-letter state with evidence.
- Restart replay reconstructs the same run state and resumes from the correct phase.

## Queued next waves

### Wave 2: Execute real workers safely

Goal: replace model-only runtime specs with supervised child processes and enforce policy before side effects.

Deliverables:

- Child-process launcher with pid, command identity, working directory, readiness, health, restart, and stop events.
- Stdout/stderr capture with redaction before logs become durable evidence.
- Sandbox runner enforcing command identity, filesystem roots, denied roots, network host allowlist, and credential-name-only injection.
- Approval queue for risky functions before execution.
- Worker heartbeat tied to active leases.

Acceptance tests:

- Starting a sample worker records pid, command identity, safe environment key names, working directory, and `runtime.started`.
- A readiness signal moves worker health from `starting` to `ready`.
- A crashed worker records sanitized logs and restarts only up to policy limits.
- Stopping a worker terminates the process group and emits `runtime.stopped`.
- Unapproved commands, denied paths, disallowed hosts, unresolved approvals, and raw secret values are blocked or redacted before persistence.

### Wave 3: Expose the operating surface

Goal: make runs inspectable through CLI, API, and control-plane panes without reading internal storage directly.

Deliverables:

- `runs watch` for ordered state transitions.
- `approvals list/show/approve/reject` with sanitized payload summaries.
- `workers status` with health, restart count, heartbeat, current leases, and failure reason.
- `queues readiness` with schema/migration ownership, heartbeat-to-lease call-site state, concurrency policy observability, and live database E2E lifecycle.
- Event stream endpoint or adapter method with cursor and pagination semantics.
- Workspace session persistence and restore.
- Evidence bundle export for events, logs, outputs, approvals, artifacts, and child summaries.

Acceptance tests:

- Watching a run prints transitions in event order without sequence gaps.
- Approval commands show blocked runs without credential payloads.
- Worker status shows live health and current leases from persisted state.
- Queue readiness shows which queue/store surfaces are modeled locally, which checks are gated by live database E2E, and which adapter wiring remains production work.
- Restoring a workspace session opens the same run, trace, terminal, worktree, worker catalog, and approval panes.
- Evidence export contains enough review material to approve or reject without rerunning work.

### Wave 4: End-to-end parity drill

Goal: prove the full Hermes-style parallel-agent operating model under real failure and review conditions.

Deliverables:

- A parent workflow with planner, parallel builders, writer, reviewer, approval gate, sandboxed command worker, storage, and evidence export.
- Separate workspace or worktree resources per child worker.
- Fanout, fanin, cancellation, rejection, retry, restart, dead-letter, and approval scenarios.
- Reviewer handoff that verifies diffs, tests, public-copy guard, policy evidence, and exported artifacts.

Acceptance tests:

- Parent run does not complete until required children complete or a policy-defined failure path is chosen.
- Parallel child workers cannot write outside assigned workspace roots.
- Reviewer starts only after required child handoffs are available.
- Review-required blocks prevent merge or release until resolved.
- Parent evidence bundle includes child summaries, metadata, logs, approvals, artifacts, and reviewer decision.

### Wave 5: Production hardening and public claim lock

Goal: make the claim safe: OpenView is not just contract-complete, it can operate repeatable agent workflows.

Deliverables:

- Migration path for local dev store and production-ready store abstraction.
- Compatibility checks for worker manifests and workflow versions.
- Default dashboards or CLI recipes for common workflows: code review, issue triage, release review, docs update, incident response, sandboxed command execution.
- Security and failure-mode review for secrets, policy bypasses, stale leases, replay gaps, and artifact leakage.
- Public docs that describe verified behavior only.

Acceptance tests:

- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- Store migrations preserve existing run history.
- Worker manifest compatibility rejects unsupported schema versions with clear errors.
- A realistic workflow runs twice from clean state with deterministic evidence shape.
- Public docs pass copy guard and avoid claims not backed by tests or recorded drills.

## Cross-wave acceptance suite

Run these before calling any wave complete:

```bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/check_public_copy.sh
```

Minimum evidence to attach to each wave handoff:

- exact commands run;
- pass/fail output summary;
- changed files;
- new or updated test names;
- reviewer notes for safety-sensitive behavior;
- known gaps that remain queued for later waves.

## Risks and mitigations

| Risk | Why it matters | Mitigation |
|---|---|---|
| Docs get ahead of code | Public parity claims become misleading. | Keep this tracker status-based and require test names plus command output before marking implementation done. |
| Multiple workers edit overlapping files | Active board work can overwrite or confuse ownership. | Each worker owns explicit files; this task owns only this tracker. Review `git status --short` before and after edits. |
| Queue semantics are underspecified | Retry, visibility timeout, stale lease, and dead-letter behavior can become inconsistent. | Treat queue protocol tests as Wave 1 blockers, not cleanup. |
| Replay diverges from snapshots | Restart safety depends on event history being authoritative. | Require replay tests that compare reconstructed state to stored run snapshots. |
| Sandbox checks happen after side effects | Policy becomes audit-only instead of protective. | Wave 2 must enforce command, path, network, credential, and approval checks before spawn or external access. |
| Secrets leak into durable evidence | Logs and event history are long-lived. | Store credential names only; add redaction tests for logs, events, outputs, runtime metadata, and evidence bundles. |
| Control plane reads private internals | UI/CLI can become a special-case view that hides API gaps. | Wave 3 surfaces must render from public API or adapter methods only. |
| Evidence export is incomplete | Reviewers have to rerun work, breaking durable review flow. | Include child summaries, test logs, approvals, policy decisions, outputs, artifacts, and reviewer comments in export tests. |
| Public-copy guard fails because of adjacent work | This tracker can be clean while another unowned doc breaks publication. | Run the guard and report exact failing paths; do not edit unowned files from this task. |

## Tracker maintenance rules

- Update this file only when a wave changes state, a test becomes authoritative, or a new risk appears.
- Do not mark a wave done from intent, design, or unreviewed diffs.
- Prefer concrete test names and command output over prose summaries.
- Keep restricted public references out of this file.
- If another worker changes parity scope, add a new row here instead of rewriting their document.
