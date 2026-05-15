#!/usr/bin/env node
import { createOpenView, permission } from "../core/index.js";

const command = process.argv[2] ?? "help";

if (command === "help" || command === "--help" || command === "-h") {
  console.log(`OpenView CLI

Usage:
  openview demo:basic       Run a local multi-agent demo
  openview demo:approval    Run a local approval-gated demo
  openview inspect:worker   Print an example worker registry entry
`);
  process.exit(0);
}

if (command === "demo:basic") {
  const view = createOpenView();
  view.registerAgent({
    id: "planner",
    name: "Planner",
    instructions: "Plan scoped work.",
    permissions: [permission.readOnly()],
    handler: async ({ input }) => ({ plan: [`plan:${input.goal}`] }),
  });
  view.registerAgent({
    id: "builder",
    name: "Builder",
    instructions: "Execute a scoped step.",
    permissions: [permission.readOnly()],
    handler: async ({ state }) => ({ shipped: state.plan }),
  });
  const run = await view.startRun({ goal: "open agent view", agents: ["planner", "builder"] });
  console.log(JSON.stringify({ status: run.status, output: run.output, events: run.events.length }, null, 2));
  process.exit(0);
}

if (command === "demo:approval") {
  const view = createOpenView();
  view.registerAgent({
    id: "writer",
    name: "Writer",
    instructions: "Write only after approval.",
    permissions: [permission.writeWorkspace({ paths: ["README.md"], approval: "required" })],
    handler: async () => ({ changed: true }),
  });
  const blocked = await view.startRun({ goal: "edit docs", agents: ["writer"] });
  console.log(JSON.stringify({ status: blocked.status, approval: blocked.approvals[0] }, null, 2));
  process.exit(0);
}

if (command === "inspect:worker") {
  const view = createOpenView();
  const worker = view.registerWorker({
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
  console.log(JSON.stringify(worker, null, 2));
  process.exit(0);
}

console.error(`Unknown command: ${command}`);
process.exit(1);
