import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const banned = [
  "sta" + "blyai",
  "github.com/" + "sta" + "blyai",
  "github.com/" + "iii" + "-hq",
  "iii" + "-hq/" + "iii",
  "iii" + "-hq/" + "workers",
];
const exactBanned = [new RegExp("\\b" + "or" + "ca" + "\\b", "i")];
const include = ["README.md", "docs", "packages", "examples", "scripts"];
const violations = [];

function walk(path) {
  const stat = statSync(path);
  if (stat.isDirectory()) {
    for (const name of readdirSync(path)) {
      if (["node_modules", "dist", ".git", ".research"].includes(name)) continue;
      walk(join(path, name));
    }
    return;
  }
  if (!/\.(md|ts|js|mjs|json|yaml|yml)$/.test(path)) return;
  const text = readFileSync(path, "utf8").toLowerCase();
  for (const term of banned) {
    if (text.includes(term)) violations.push(`${path.replace(root + "/", "")}: ${term}`);
  }
  for (const pattern of exactBanned) {
    if (pattern.test(text)) violations.push(`${path.replace(root + "/", "")}: ${pattern}`);
  }
}

for (const name of include) walk(join(root, name));
if (violations.length) {
  console.error("Public copy contains restricted references:\n" + violations.join("\n"));
  process.exit(1);
}
console.log("Public copy check passed");
