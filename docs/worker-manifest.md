# Worker Manifest Design

A worker manifest is the contract between OpenView, developers, and operators.

## Minimum fields

```yaml
id: repo.reader
name: Repository Reader
version: 0.1.0
runtime: local
capabilities:
  - files.read
  - code.search
sideEffects: read-only
health: online
inputSchema:
  type: object
  required: [query]
outputSchema:
  type: object
  required: [matches]
approvalRequired: false
```

## Why these fields matter

- `id` gives tools and agents a stable dependency target.
- `version` lets in-flight runs pin capability behavior.
- `runtime` tells OpenView where execution happens.
- `capabilities` lets agents discover narrow workers.
- `sideEffects` lets policy and UI explain risk.
- `inputSchema` and `outputSchema` make composition safer.
- `health` gives the Agent View an operational signal.
- `approvalRequired` allows safe defaults for risky workers.

## Recommended operational metadata

- owner/team;
- source/provenance;
- install command;
- deploy target;
- cost hint;
- latency hint;
- failure modes;
- observability endpoint;
- environment variables needed, without secret values;
- network/file/process scopes.
