# Worker Catalog

OpenView workers are narrow, typed capability providers. They should be small enough to reason about and explicit enough to review before an agent uses them.

## Required manifest fields

- `schema_version`
- `name`
- `display_name`
- `version`
- `description`
- `language`
- `deploy_kind`
- `binary_name`
- `targets`
- `functions`
- `triggers`
- `resources`
- `dependencies`
- `default_config`
- `security`
- `sandbox`

## Core workers

### `approval.gate`

Owns approval requests and resolutions. Used before risky functions such as command execution, writes, network calls, credential use, or external side effects.

### `shell.sandbox`

Runs scoped commands under a sandbox profile. Must declare filesystem roots, process permissions, network policy, and approval requirements.

The Rust foundation can evaluate sandbox policy without executing anything:

- read and write checks are scoped by declared roots and denied roots;
- command checks use exact command identities, not shell command lines;
- network checks require network mode plus an exact allowed host;
- secrets are referenced by configured names only, never by raw values.

### `turn.orchestrator`

Owns durable agent turns, state transitions, function-call lifecycle, transcript, event stream, and teardown.

### AgentView CLI runners

AgentView treats local coding tools as iii function-call workers with one
lifecycle contract instead of special UI cases. The current Rust manifests and
invocation contracts cover the expanded catalog below; live runtime parity still
depends on installing the upstream CLI and verifying each iii worker package.

| Worker | Binary | Contract |
|---|---|---|
| `codex.agent` | `codex` | `spawn_session`, `send_prompt`, `stream_events`, `resolve_approval` for Codex CLI worktree runs. |
| `claude-code.agent` | `claude` | `spawn_session`, `send_prompt`, `stream_events`, `resolve_approval` for Claude Code worktree runs. |
| `hermes.agent` | `hermes` | Profile/session-aware Hermes runs with skills/toolsets and approval/event streams. |
| `openclaw.agent` | `openclaw` | OpenClaw sessions behind the same AgentView function-call contract. |
| `opencode.agent` | `opencode` | `spawn_session`, `send_prompt`, `stream_events`, `resolve_approval` for OpenCode worktree runs. |
| `gemini-cli.agent` | `gemini` | Gemini CLI sessions with normalized event streaming and approval resolution. |
| `goose.agent` | `goose` | Goose sessions scoped to worktrees and replayable streams. |
| `aider.agent` | `aider` | Aider coding runs in selected worktrees with approval gates. |
| `openhands.agent` | `openhands` | OpenHands CLI/session runs through AgentView. |
| `crush.agent` | `crush` | Crush runs with the shared session/event/approval shape. |
| `qwen-code.agent` | `qwen` | Qwen Code sessions through the shared runner wrapper. |
| `cursor-agent.agent` | `cursor-agent` | Cursor Agent sessions as AgentView worker runs. |
| `amp.agent` | `amp` | Amp sessions wrapped as AgentView worker runs. |

Each runner depends on the same iii primitives:

- `git.worktree` creates or selects the scoped workspace for the run.
- `shell.sandbox` or the process sandbox launches the local CLI binary.
- `approval.gate` blocks risky file, process, network, credential, or external side effects.
- iii streams record normalized stdout, stderr, tool-call, approval, and status events.
- `session-tree` stores transcript/history, branches, and resumable session paths.
- `iii-database` and `iii-queue` provide durable state, task delivery, retries, leases, and dead-letter evidence.
- credential and model-provider workers resolve named credentials and model/provider routing without storing raw secret values in manifests.

The worker install path is `iii worker add ...`, not copying manifests by hand.
Install the substrate workers, then add runner wrappers as needed:

```bash
iii worker add git-worktree
iii worker add shell
iii worker add approval-gate
iii worker add session-tree
iii worker add iii-database
iii worker add iii-queue
iii worker add auth-credentials
iii worker add provider-router
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

The wrapper install configures the iii worker package. Operators still install
the upstream CLI binary separately and expose only approved environment variable
names to the worker runtime.

### `models.router`

Routes model calls to provider workers while checking budget and provider policy.

### `policy.guard`

Evaluates requested capabilities against denylist/default-deny rules.

### `storage.objects`

Stores artifacts, screenshots, logs, payloads, and generated resources.

## Extended workers to add next

- `session.state`
- `hook.fanout`
- `credentials.auth`
- `mcp.bridge`
- `database.query`
- `browser.session`
- `terminal.pty`
- `git.worktree`
- `github.issue_pr`
- `ci.status`
- `budget.ledger`
- `sensor.watch`

## iii primitive mapping

OpenView worker manifests should stay typed and reviewable while the runtime adapter routes work to iii primitives:

| OpenView worker/resource | iii target | Operator contract |
|---|---|---|
| `approval.gate` | `approval::list_pending`, `approval::resolve`, plus `hook-fanout::publish_collect` | Risky work blocks before side effects; approvals, rejections, expirations, and comments are replayable evidence. |
| `shell.sandbox` | shell sandbox worker | Commands run only inside declared filesystem, process, network, and credential policy. |
| `turn.orchestrator` | `run::start_and_wait` for Hermes agent turns | Agent execution is a bounded run step with transcript, output, status, and failure evidence. |
| AgentView runner workers (`codex.agent`, `claude-code.agent`, `hermes.agent`, `openclaw.agent`, `opencode.agent`, `gemini-cli.agent`, `goose.agent`, `aider.agent`, `openhands.agent`, `crush.agent`, `qwen-code.agent`, `cursor-agent.agent`, `amp.agent`) | `git.worktree` + shell/process sandbox + `approval::resolve` + iii streams + `session-tree` + `iii-database`/`iii-queue` + credential/model providers | AgentView can run multiple agent CLIs side-by-side, each in its own worktree, with approvals, durable session history, and replayable output. Current repo support is manifest/invocation contracts first, with live runtime parity added per runner. |
| `session.state` | `session-tree::*` | Transcript/history, branches, compaction markers, and exportable active paths survive restart. |
| `storage.objects` and `Database` | `iii-database` transaction fallback | Runs, tasks, checkpoints, waits, idempotency keys, leases, retry state, and events are durable even when queue/stream workers are unavailable. |
| `EventStream` | `stream::set`, `stream::list` | UI panes and CLIs see ordered event slices and live updates. |
| `HookTopic` | `hook-fanout::publish_collect` | Policy, approval, budget, and evidence subscribers can inspect or block calls consistently. |
| control-plane harness | harness worker | Local UI/event streaming composes the same worker surfaces operators inspect. |

Implemented contract/runtime surfaces:

- `QueueTask` and `OpenViewControlApi` cover enqueue, compatible polling, lease ownership, visibility timeout, current-lease complete/fail, stale lease rejection, retry backoff, terminal failed/dead-letter state, explicit requeue, and expired-lease recovery.
- `LocalRunStore` covers workflow definitions, run snapshots, run events, approvals embedded in runs, worker runtimes, queue tasks, artifacts, restart snapshots, and evidence bundle summaries.
- Worker runtime heartbeat is modeled in supervision events; `OpenViewControlApi::heartbeat_worker_leases` extends active owned queue leases from the same worker heartbeat, missed heartbeats still fall back to visibility-timeout recovery, and the process supervisor emits a sanitized heartbeat event naming that control API call-site.
- Concurrency is observable through `QueueConcurrencySnapshotResponse`: local policy reports active leases, visible available tasks, requested max concurrency, and limit blocking; delegated iii-queue policy is represented without claiming local enforcement.
- `cargo run -p openview-cli -- queues readiness` exposes a deterministic operator snapshot for queue schema records, migration ownership, worker heartbeat-to-lease wiring, concurrency policy observability, and the live iii database E2E lifecycle without reading credentials.

The adapter must not require the `iii` CLI to be on `PATH`; local integration should use a configured binary path or API client. Tests for worker manifests should keep passing without a live iii runtime. `scripts/e2e_iii_runtime.sh` is the explicit live check for the iii engine and harness worker stack.

For database-backed task-state verification, run:

```bash
OPENVIEW_III_E2E_DATABASE=1 OPENVIEW_III_MANAGED=1 scripts/e2e_iii_runtime.sh
```

That mode starts `iii-database`, applies the owned durable queue schema bootstrap, verifies durable tables/indexes/columns, and validates transaction/query/execute paths for task lease, heartbeat, recovery, retry, and dead-letter state. The default SQLite file is `$III_E2E_DEMO_DIR/openview-e2e.db`, which resolves to `/private/tmp/openview-iii-e2e/openview-e2e.db` unless overridden.

## Worker review checklist

Before a worker is accepted:

- Does every function have a schema?
- Does every risky function require approval or policy evaluation?
- Are resources declared?
- Are dependencies explicit?
- Can the worker print a manifest without connecting to a backend?
- Does the worker shut down cleanly?
- Does the test suite cover manifest, policy, and one happy path?
