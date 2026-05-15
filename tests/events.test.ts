import assert from "node:assert/strict";
import test from "node:test";
import { createEventStore } from "../packages/core/index.js";

test("event store assigns monotonic ids and supports resume cursors", () => {
  const events = createEventStore();
  events.append("run.started", { runId: "run_1" });
  events.append("agent.started", { agentId: "planner" });
  events.append("agent.completed", { agentId: "planner" });

  assert.deepEqual(events.after(1).map((event) => event.type), ["agent.started", "agent.completed"]);
  assert.equal(events.all()[2].sequence, 3);
});
