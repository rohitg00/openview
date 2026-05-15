import { createOpenView, permission } from "../packages/core/index.js";

const view = createOpenView();

view.registerAgent({
  id: "planner",
  name: "Planner",
  instructions: "Turn a goal into scoped worker steps.",
  permissions: [permission.readOnly()],
  handler: async ({ input }) => ({ plan: [`Research ${input.goal}`, `Build ${input.goal}`, `Verify ${input.goal}`] }),
});

view.registerAgent({
  id: "executor",
  name: "Executor",
  instructions: "Run one scoped action at a time.",
  permissions: [permission.readOnly()],
  handler: async ({ state }) => ({ completed: state.plan }),
});

const run = await view.startRun({ goal: "a worker-backed demo", agents: ["planner", "executor"] });
console.log(JSON.stringify({ status: run.status, output: run.output, events: run.events.length }, null, 2));
