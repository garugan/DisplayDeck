import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

const cwd = fileURLToPath(new URL("../", import.meta.url));
const options = { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] };
console.log("READ_ONLY RC environment check; no build or installer execution.");
console.log("Repository:", cwd);
console.log("HEAD:", execFileSync("git", ["rev-parse", "HEAD"], options).trim());
console.log("Working tree (empty = clean):\n" + execFileSync("git", ["status", "--short"], options));
console.log("Read-only source:", execFileSync("git", ["rev-parse", "--verify", "e598cc4d08a37ec6815a80a2f864f562c59ed8e6^{commit}"], options).trim());
console.log("Platform:", process.platform, process.arch);
console.log("Node:", process.version);
console.log("Rust:", execFileSync("rustc", ["--version"], options).trim());
console.log("Cargo:", execFileSync("cargo", ["--version"], options).trim());
console.log("Local Tauri CLI present:", existsSync(new URL("../node_modules/@tauri-apps/cli/tauri.js", import.meta.url)));
