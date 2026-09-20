import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import test from "node:test";

test("unsupported host rejects RC build before creating artifacts", { skip: process.platform === "win32" }, () => {
  const candidate = new URL("../artifacts/v0.1.0-rc3", import.meta.url);
  const existed = existsSync(candidate);
  const result = spawnSync(process.execPath, [new URL("rc-build.mjs", import.meta.url).pathname], { encoding: "utf8" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Run only on Windows; no files changed/);
  assert.equal(existsSync(candidate), existed);
});

test("status check rejects unsupported host without starting PowerShell", { skip: process.platform === "win32" }, () => {
  const result = spawnSync(process.execPath, [new URL("rc-status.mjs", import.meta.url).pathname], { encoding: "utf8" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Run only on Windows; no files changed/);
});

test("build output reaches log on success and failures remain failures", async () => {
  const { runLogged } = await import("./rc-build.mjs");
  const { mkdtempSync, readFileSync, rmSync } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");
  const dir = mkdtempSync(join(tmpdir(), "displaydeck-log-"));
  try {
    const log = join(dir, "ok.log");
    await runLogged(process.execPath, ["-e", "console.log('stdout check'); console.error('stderr check')"], {}, log);
    assert.match(readFileSync(log, "utf8"), /stdout check/);
    assert.match(readFileSync(log, "utf8"), /stderr check/);
    await assert.rejects(runLogged(process.execPath, ["-e", "process.exit(7)"], {}, join(dir, "fail.log")), /exit=7/);
    await assert.rejects(runLogged(process.execPath, ["-e", "process.exit(0)"], {}, log), /EEXIST/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
