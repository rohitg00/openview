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

### `turn.orchestrator`

Owns durable agent turns, state transitions, function-call lifecycle, transcript, event stream, and teardown.

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

## Worker review checklist

Before a worker is accepted:

- Does every function have a schema?
- Does every risky function require approval or policy evaluation?
- Are resources declared?
- Are dependencies explicit?
- Can the worker print a manifest without connecting to a backend?
- Does the worker shut down cleanly?
- Does the test suite cover manifest, policy, and one happy path?
