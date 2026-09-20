import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

assert.equal(process.platform, "win32", "Run only on Windows; no files changed.");
const rc = fileURLToPath(new URL("../artifacts/v0.1.0-rc2/", import.meta.url));
console.log("READ_ONLY snapshot. No build, process termination, or artifact changes.");
console.log("Possible build processes (may include unrelated builds):");
console.log(execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command",
  "$ErrorActionPreference='Stop'; Get-CimInstance Win32_Process -Filter \"Name='node.exe' OR Name='cargo.exe' OR Name='rustc.exe' OR Name='link.exe' OR Name='makensis.exe' OR Name='tauri.exe' OR Name='rustup.exe'\" | Select-Object Name,ProcessId,ParentProcessId,CreationDate | Format-Table -AutoSize | Out-String -Width 200",
], { encoding: "utf8", timeout: 30000 }));
for (const name of ["attempt.json", "candidate-manifest.json", "SHA256SUMS.txt", "DisplayDeck_0.1.0-rc2_x64-setup.exe", "source/target/release/bundle/nsis/DisplayDeck_0.1.0_x64-setup.exe", "build.log"]) {
  const path = join(rc, name);
  if (!existsSync(path)) {
    console.log("MISSING:", name);
    continue;
  }
  const info = statSync(path);
  console.log(name, "bytes:", info.size, "modified:", info.mtime.toISOString());
  if (name.endsWith(".json") || name.endsWith(".txt")) console.log(readFileSync(path, "utf8"));
}
const log = join(rc, "build.log");
if (existsSync(log)) {
  console.log("Last 60 build.log lines:");
  console.log(execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command",
    "Get-Content -LiteralPath $env:DISPLAYDECK_RC_LOG -Tail 60 -ErrorAction Stop",
  ], { encoding: "utf8", env: { ...process.env, DISPLAYDECK_RC_LOG: log }, timeout: 30000 }));
}
console.log("Share this output. Do not rebuild, delete files, or run the installer.");
