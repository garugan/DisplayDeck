import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

if (process.platform !== "win32" || process.arch !== "x64") {
  throw new Error("The DisplayDeck NSIS package must be built on x64 Windows.");
}

run("cargo.exe", [
  "build",
  "-p",
  "displaydeck-safety",
  "--bin",
  "displaydeck-actor",
  "--release",
]);

const binaries = resolve(root, "src-tauri", "binaries");
mkdirSync(binaries, { recursive: true });
copyFileSync(
  resolve(root, "target", "release", "displaydeck-actor.exe"),
  resolve(binaries, "displaydeck-actor-x86_64-pc-windows-msvc.exe"),
);

run(process.execPath, [
  resolve(root, "node_modules", "@tauri-apps", "cli", "tauri.js"),
  "build",
  "--config",
  resolve(root, "src-tauri", "tauri.bundle.windows.conf.json"),
]);

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
