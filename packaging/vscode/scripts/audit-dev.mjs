// Audit of the build tools (devDependencies): fails on a high or critical advisory,
// except the ones listed below, each with the reason it cannot reach users.
// What the .vsix ships is audited separately at every level (npm audit --omit=dev).
import { execSync } from "node:child_process";

const ACCEPTED = {
  // picomatch ReDoS through repeated extglobs. Only @vscode/vsce reaches it, with
  // the patterns of our own .vscodeignore, when the .vsix is packed; every picomatch
  // release is affected, and npm's only fix is @vscode/vsce 3.9.2, a breaking change.
  "GHSA-fc5q-39xc-vjpf": "picomatch ReDoS: build tool only, trusted patterns, no fixed release",
};

let out;
try {
  out = execSync("npm audit --json", { encoding: "utf8", maxBuffer: 64 << 20 });
} catch (e) {
  // npm audit exits 1 when it finds anything; the report is still on stdout
  out = e.stdout;
}
const report = JSON.parse(out);
const advisories = new Map();
for (const v of Object.values(report.vulnerabilities ?? {})) {
  for (const via of v.via) {
    if (typeof via === "object" && via.url) advisories.set(via.url, via);
  }
}

let failed = 0;
const seen = new Set();
for (const a of advisories.values()) {
  if (a.severity !== "high" && a.severity !== "critical") continue;
  const id = a.url.split("/").pop();
  seen.add(id);
  if (ACCEPTED[id]) {
    console.log(`::warning::${a.name} (${a.severity}) ${a.url} accepted: ${ACCEPTED[id]}`);
  } else {
    console.log(`::error::${a.name} (${a.severity}): ${a.title} (${a.url})`);
    failed++;
  }
}
for (const id of Object.keys(ACCEPTED)) {
  if (!seen.has(id)) console.log(`::notice::${id} no longer reported: remove it from scripts/audit-dev.mjs`);
}
if (failed) {
  console.log(`${failed} high or critical advisories in the build tools`);
  process.exit(1);
}
console.log("build tools: no high or critical advisory besides the accepted ones");
