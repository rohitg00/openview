import assert from "node:assert/strict";
import test from "node:test";
import { createOpenView, permission, type AgentHandler } from "../packages/core/index.js";

test("runs a sequential agent graph and records events", async () => {
  const app = createOpenView();
  app.registerAgent({
    id: "planner",
    name: "Planner",
    instructions: "Create a route",
    permissions: [permission.readOnly()],
    handler: async ({ input, emit }) => {
      emit("agent.message", { text: `plan:${input.goal}` });
      return { route: ["worker"] };
    },
  });
  app.registerAgent({
    id: "worker",
    name: "Worker",
    instructions: "Execute one scoped task",
    permissions: [permission.readOnly()],
    handler: async ({ state }) => {
      const route = state.route as string[];
      return { done: route[0] === "worker" };
    },
  });

  const run = await app.startRun({ goal: "ship", agents: ["planner", "worker"] });

  assert.equal(run.status, "completed");
  assert.deepEqual(run.output, { done: true });
  assert.equal(run.steps.length, 2);
  assert.equal(run.events.at(0)?.type, "run.started");
  assert.equal(run.events.at(-1)?.type, "run.completed");
});

test("blocks write-like permissions until approved", async () => {
  const app = createOpenView();
  const writer: AgentHandler = async () => ({ changed: true });
  app.registerAgent({
    id: "writer",
    name: "Writer",
    instructions: "Mutate a workspace",
    permissions: [permission.writeWorkspace({ paths: ["README.md"], approval: "required" })],
    handler: writer,
  });

  const blocked = await app.startRun({ goal: "edit", agents: ["writer"] });
  assert.equal(blocked.status, "blocked");
  assert.equal(blocked.approvals.length, 1);
  assert.equal(blocked.steps[0].status, "waiting_for_approval");

  const approved = await app.approveAndResume(blocked.id, blocked.approvals[0].id, "Looks safe");
  assert.equal(approved.status, "completed");
  assert.deepEqual(approved.output, { changed: true });
});

test("worker registry exposes side effects, schemas, and health", () => {
  const app = createOpenView();
  app.registerWorker({
    id: "repo.reader",
    name: "Repository Reader",
    version: "0.1.0",
    runtime: "local",
    capabilities: ["files.read", "code.search"],
    sideEffects: "read-only",
    inputSchema: { type: "object", required: ["query"] },
    outputSchema: { type: "object", required: ["matches"] },
    health: "online",
  });

  const entry = app.registry.workers.get("repo.reader");
  assert.equal(entry?.sideEffects, "read-only");
  assert.equal(entry?.health, "online");
  assert.deepEqual(entry?.capabilities, ["files.read", "code.search"]);
});
