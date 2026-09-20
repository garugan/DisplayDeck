import assert from "node:assert/strict";
import test from "node:test";
import { verifyRc } from "./rc-verify.mjs";

test("RC3 verification rejects wrong size and same-size wrong bytes", () => {
  assert.throws(() => verifyRc(Buffer.alloc(1)), /size mismatch/);
  assert.throws(() => verifyRc(Buffer.alloc(2160643)), /SHA256 mismatch/);
});
