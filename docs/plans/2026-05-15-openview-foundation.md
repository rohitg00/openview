# OpenView Foundation Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Build the first public foundation of OpenView: a typed local orchestration core with worker registry, approvals, events, examples, and detailed docs.

**Architecture:** Local-first TypeScript library. Agents and workers are registered in a typed registry. Runs execute agent steps, block on approval-required permissions, emit monotonic events, and can resume after approval. A narrow backend adapter keeps the path open for 3i-compatible durable execution.

**Tech Stack:** TypeScript, Node 22 test runner, npm scripts, strict `tsc`.

---

### Task 1: Create test-first scaffold

**Objective:** Establish package metadata, TypeScript config, and failing tests before implementation.

**Files:**
- Create: `package.json`
- Create: `tsconfig.json`
- Create: `tests/orchestrator.test.ts`
- Create: `tests/events.test.ts`

**Verification:** `npm test` fails because `packages/core/index.js` does not exist.

### Task 2: Implement core primitives

**Objective:** Add typed run, agent, worker, tool, permission, approval, and event models.

**Files:**
- Create: `packages/core/types.ts`
- Create: `packages/core/events.ts`
- Create: `packages/core/permissions.ts`
- Create: `packages/core/registry.ts`
- Create: `packages/core/orchestrator.ts`
- Create: `packages/core/index.ts`

**Verification:** `npm test` passes all orchestration and event tests.

### Task 3: Add backend adapter boundary

**Objective:** Define a minimal backend interface and 3i-compatible HTTP adapter shape without coupling public UX to backend internals.

**Files:**
- Create: `packages/core/backends.ts`
- Modify: `packages/core/index.ts`

**Verification:** `npm run typecheck` passes.

### Task 4: Add SDK and examples

**Objective:** Provide a small client wrapper and two runnable examples.

**Files:**
- Create: `packages/sdk/index.ts`
- Create: `examples/basic-agent.ts`
- Create: `examples/approval-gate.ts`

**Verification:** `npm run build`, then run compiled examples.

### Task 5: Add documentation and copy guard

**Objective:** Publish a detailed README, architecture docs, worker manifest design, research notes, and a copy guard that prevents private research target names from leaking into public files.

**Files:**
- Create: `README.md`
- Create: `docs/architecture.md`
- Create: `docs/worker-manifest.md`
- Create: `docs/research-notes.md`
- Create: `scripts/check-public-copy.mjs`

**Verification:** `npm run lint:copy` passes.

### Task 6: Final validation and publish branch

**Objective:** Run checks, inspect diff, commit as `rohitg00`, and push only to `rohitg00/openview`.

**Commands:**

```bash
npm run check
npm run lint:copy
git status --short
git log -1 --format='%an <%ae> %s'
```
