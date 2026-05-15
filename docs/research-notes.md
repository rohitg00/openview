# Research Notes

These notes distill private/local research into source-neutral product requirements for OpenView. Public project copy avoids naming individual reference projects and instead captures the repeated infrastructure patterns.

## Signals that shaped OpenView

### Durable execution

Recent agent infrastructure is moving toward long-running, resumable worker/function systems. OpenView therefore separates the local SDK from the backend boundary and treats runs, steps, retries, approvals, and streams as durable concepts.

### Worker registries

Capability registries are becoming control planes. A useful registry needs more than a name and URL. It should include schemas, permissions, side-effect class, runtime, health, ownership, version, cost hints, latency expectations, failure modes, and install/deploy instructions.

### Approval and sandboxing

The safest default for agent systems is read-only. File writes, shell commands, network calls, external side effects, and secret access should require explicit scope and, when appropriate, human approval.

### Semantic observability

Plain logs are not enough. Agent systems need timelines that capture decisions, tool calls, worker execution, state changes, memory access, handoffs, approvals, outputs, errors, cost, and latency.

### Multi-surface sessions

Users increasingly expect to start a task in one place and monitor or approve it somewhere else. OpenView’s event model is cursor-friendly so a web, desktop, mobile, or CLI surface can reconnect and continue watching a run.

## Product requirements carried into the implementation

- Agent and worker are separate concepts.
- Worker capabilities must be inspectable before use.
- Permissions must live next to the agent/tool that needs them.
- Approval requests must record reason and resolution.
- Runs must preserve steps and event history.
- Event sequences must support resume cursors.
- Backend integration must not leak backend details into the local developer experience.
