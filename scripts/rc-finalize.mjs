import assert from "node:assert/strict";
import { readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { verifyRc } from "./rc-verify.mjs";

assert.equal(process.platform, "win32", "Run only on Windows; no files changed.");
const root = fileURLToPath(new URL("../", import.meta.url));
const out = join(root, "release", "v0.1.0");
const docs = join(root, "docs", "release");
const approvedText = readFileSync(join(docs, "v0.1.0-release-manifest.json"), "utf8");
const approved = JSON.parse(approvedText);
const local = JSON.parse(readFileSync(join(out, "release-manifest.json"), "utf8"));
for (const key of ["candidate", "sourceCommit", "artifact", "size", "sha256", "versions", "smokeTest", "artifactIdentity"]) {
  assert.deepEqual(local[key], approved[key], `Release metadata mismatch: ${key}`);
}
assert.equal(approved.gateC, "APPROVED");
assert.ok(["PENDING", "APPROVED"].includes(local.gateC));
assert.equal(verifyRc(readFileSync(join(out, approved.artifact))), approved.sha256);
assert.equal(readFileSync(join(out, "SHA256SUMS.txt"), "utf8").trim(), `${approved.sha256}  ${approved.artifact}`);
const smoke = readFileSync(join(docs, "v0.1.0-smoke-test.md"));
const notes = readFileSync(join(docs, "v0.1.0-release-notes.md"));
// Publish the approved manifest last; a failed metadata update remains retryable.
for (const [name, contents] of [["smoke-test.md", smoke], ["RELEASE_NOTES.md", notes], ["release-manifest.json", approvedText]]) {
  const temporary = join(out, `${name}.pending`);
  writeFileSync(temporary, contents, { flag: "wx" });
  renameSync(temporary, join(out, name));
}
assert.equal(verifyRc(readFileSync(join(out, approved.artifact))), approved.sha256);
console.log(`RELEASED v0.1.0 | Gate C APPROVED | SHA256 ${approved.sha256}`);
console.log("Installer unchanged. Local release metadata finalized.");
