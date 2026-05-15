import { createOpenView, permission } from "../packages/core/index.js";

const view = createOpenView();

view.registerAgent({
  id: "workspace-writer",
  name: "Workspace Writer",
  instructions: "Prepare a file change after approval.",
  permissions: [permission.writeWorkspace({ paths: ["docs/decision.md"], approval: "required" })],
  handler: async () => ({ artifact: "docs/decision.md", summary: "change prepared" }),
});

const blocked = await view.startRun({ goal: "prepare a decision note", agents: ["workspace-writer"] });
console.log(blocked.status, blocked.approvals[0]);

const completed = await view.approveAndResume(blocked.id, blocked.approvals[0].id, "approved in local demo");
console.log(completed.status, completed.output);
