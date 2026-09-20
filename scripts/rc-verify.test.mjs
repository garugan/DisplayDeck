import assert from "node:assert/strict";
import test from "node:test";
import { verifyRc } from "./rc-verify.mjs";

test("RC3 verification rejects wrong size and same-size wrong bytes", () => {
  assert.throws(() => verifyRc(Buffer.alloc(1)), /size mismatch/);
  assert.throws(() => verifyRc(Buffer.alloc(2160643)), /SHA256 mismatch/);
});

test("finalizer rejects non-Windows before writing release metadata", { skip: process.platform === "win32" }, async () => {
  const { spawnSync } = await import("node:child_process");
  const { fileURLToPath } = await import("node:url");
  const result = spawnSync(process.execPath, [fileURLToPath(new URL("rc-finalize.mjs", import.meta.url))], { encoding: "utf8" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Run only on Windows; no files changed/);
});
