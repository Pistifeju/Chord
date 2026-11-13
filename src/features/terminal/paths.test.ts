// Tests for the terminal's dropped path rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { shellPath } from "./paths.ts";

test("a path with spaces or quotes is quoted for the shell, a plain one is not", () => {
  assert.equal(shellPath("/tmp/a-b_c.txt"), "/tmp/a-b_c.txt");
  assert.equal(shellPath("/tmp/my file.txt"), "'/tmp/my file.txt'");
  assert.equal(shellPath("/tmp/it's"), "'/tmp/it'\\''s'");
});
