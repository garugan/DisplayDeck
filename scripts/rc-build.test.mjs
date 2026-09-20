import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import test from "node:test";

test("unsupported host rejects RC build before creating artifacts", { skip: process.platform === "win32" }, () => {
  const candidate = new URL("../artifacts/v0.1.0-rc2", import.meta.url);
  const existed = existsSync(candidate);
  const result = spawnSync(process.execPath, [new URL("rc-build.mjs", import.meta.url).pathname], { encoding: "utf8" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Run only on Windows; no files changed/);
  assert.equal(existsSync(candidate), existed);
});
