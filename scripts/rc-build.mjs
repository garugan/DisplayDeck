import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { closeSync, constants, copyFileSync, existsSync, mkdirSync, openSync, readFileSync, statSync, symlinkSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const source = "e598cc4d08a37ec6815a80a2f864f562c59ed8e6";
const rc = join(root, "artifacts", "v0.1.0-rc3");
const checkout = join(rc, "source");
const name = "DisplayDeck_0.1.0-rc3_x64-setup.exe";

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: root, encoding: "utf8", ...options });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, `${command} failed: ${result.stderr ?? "see build.log"}`);
  return result.stdout?.trim() ?? "";
}

export async function runLogged(command, args, options, logPath) {
  const log = openSync(logPath, "wx");
  const started = Date.now();
  const heartbeat = setInterval(() => console.log(`Build still running (${Math.floor((Date.now() - started) / 1000)}s). Waiting for build completion.`), 15000);
  try {
    await new Promise((accept, reject) => {
      const child = spawn(command, args, { ...options, stdio: ["ignore", "pipe", "pipe"] });
      let outputError;
      const forward = (stream) => (chunk) => {
        if (outputError) return;
        try {
          writeFileSync(log, chunk);
          stream.write(chunk);
        } catch (error) {
          outputError = error;
        }
      };
      child.stdout.on("data", forward(process.stdout));
      child.stderr.on("data", forward(process.stderr));
      child.on("error", reject);
      child.on("close", (code, signal) => outputError ? reject(outputError) : code === 0 ? accept() : reject(new Error(`Build failed: exit=${code}, signal=${signal}; preserve ${logPath}`)));
    });
  } finally {
    clearInterval(heartbeat);
    closeSync(log);
  }
}

async function main() {
  assert.equal(process.platform, "win32", "Run only on Windows; no files changed.");
  assert.equal(process.arch, "x64", "Run only on x64 Windows.");
  assert.equal(run("git", ["status", "--porcelain"]), "", "Working tree must be clean.");
  assert.equal(run("git", ["rev-parse", "--verify", `${source}^{commit}`]), source);
  assert.ok(!existsSync(rc), "RC directory already exists. Stop; never overwrite or rebuild this RC.");
  assert.ok(existsSync(join(root, "node_modules", "@tauri-apps", "cli", "tauri.js")), "Existing Tauri CLI required; do not install dependencies.");
  assert.equal(run("git", ["diff", source, "HEAD", "--", "package.json", "package-lock.json"]), "", "Dependency definitions differ from frozen source.");
  const versions = {
    node: process.version,
    rust: run("rustc", ["--version"]),
    cargo: run("cargo", ["--version"]),
  };
  mkdirSync(rc, { recursive: true });
  writeFileSync(join(rc, "attempt.json"), JSON.stringify({ source, versions, startedAt: new Date().toISOString() }, null, 2), { flag: "wx" });
  run("git", ["worktree", "add", "--detach", checkout, source]);
  symlinkSync(join(root, "node_modules"), join(checkout, "node_modules"), "junction");
  const logPath = join(rc, "build.log");
  console.log(`One RC build. Output: ${logPath}\nDo not rerun or delete this RC directory if the build fails.`);
  await runLogged(process.execPath, [join(checkout, "scripts", "build-windows-installer.mjs")], {
    cwd: checkout,
    env: { ...process.env, CARGO_NET_OFFLINE: "true", CARGO_TARGET_DIR: join(checkout, "target") },
  }, logPath);
  const built = join(checkout, "target", "release", "bundle", "nsis", "DisplayDeck_0.1.0_x64-setup.exe");
  const sha256 = createHash("sha256").update(readFileSync(built)).digest("hex").toUpperCase();
  const destination = join(rc, name);
  copyFileSync(built, destination, constants.COPYFILE_EXCL);
  assert.equal(createHash("sha256").update(readFileSync(destination)).digest("hex").toUpperCase(), sha256);
  writeFileSync(join(rc, "SHA256SUMS.txt"), `${sha256}  ${name}\n`, { flag: "wx" });
  assert.equal(run("git", ["diff", "--name-only", "HEAD"], { cwd: checkout }), "", "Build changed tracked source; candidate must not be used.");
  const manifest = { candidate: "v0.1.0-rc3", sourceCommit: source, artifact: name, size: statSync(destination).size, sha256, versions, buildLog: "build.log", smokeTest: "NOT_RUN", gateC: "PENDING" };
  writeFileSync(join(rc, "candidate-manifest.json"), JSON.stringify(manifest, null, 2) + "\n", { flag: "wx" });
  console.log(JSON.stringify({ ...manifest, path: destination }, null, 2));
  console.log("RC frozen. Do not build again. Do not install until the smoke procedure is recorded.");
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) await main();
