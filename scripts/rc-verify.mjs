import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export function verifyRc(bytes) {
  assert.equal(bytes.length, 2160643, "RC3 size mismatch. Stop.");
  const hash = createHash("sha256").update(bytes).digest("hex").toUpperCase();
  assert.equal(hash, "25DEFCA4CC6DA01F350CC82E1302FF0CE1783BBCEE2A88B77EE2734E0C637EFD", "RC3 SHA256 mismatch. Stop.");
  return hash;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const path = fileURLToPath(new URL("../artifacts/v0.1.0-rc3/DisplayDeck_0.1.0-rc3_x64-setup.exe", import.meta.url));
  console.log("SHA256:", verifyRc(readFileSync(path)));
  console.log("RC3 identity PASS:", path);
}
