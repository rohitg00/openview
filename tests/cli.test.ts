import assert from "node:assert/strict";
import test from "node:test";
import { execFileSync } from "node:child_process";

const node = process.execPath;

test("CLI basic demo prints a completed run", () => {
  const output = execFileSync(node, ["dist/packages/cli/index.js", "demo:basic"], { encoding: "utf8" });
  const parsed = JSON.parse(output);
  assert.equal(parsed.status, "completed");
  assert.equal(parsed.events, 6);
});

test("CLI worker inspection prints registry metadata", () => {
  const output = execFileSync(node, ["dist/packages/cli/index.js", "inspect:worker"], { encoding: "utf8" });
  const parsed = JSON.parse(output);
  assert.equal(parsed.id, "repo.reader");
  assert.equal(parsed.sideEffects, "read-only");
});
