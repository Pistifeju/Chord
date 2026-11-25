import { test } from "node:test";
import assert from "node:assert/strict";
import type { Checks, HarnessCheck, Install } from "../../ipc/bindings";
import { blockingRows, neededCommand } from "./harnessCheck.ts";

const harness = (name: string, installState: Install, starts = true, takesPick = false): HarnessCheck => ({
  name,
  title: name,
  starts,
  install: `install ${name}`,
  signIn: `login ${name}`,
  upgrade: `update ${name}`,
  installState,
  takesPick,
});

test("a row needs the install, sign-in or upgrade command, and none when ready", () => {
  assert.equal(neededCommand(harness("pi", { kind: "missing" })), "install pi");
  assert.equal(neededCommand(harness("pi", { kind: "signed_out", version: "1.0.0" })), "login pi");
  assert.equal(neededCommand(harness("pi", { kind: "outdated", version: "1.0.0", oldest: "2.0.0" })), "update pi");
  assert.equal(neededCommand(harness("pi", { kind: "ready", version: "1.0.0" })), null);
});

test("a pane shows its harness's row while it cannot start, and every row while it has none and no default exists", () => {
  const pi = harness("pi", { kind: "missing" });
  const codex = harness("codex", { kind: "signed_out", version: "1.0.0" }, true, true);
  const claude = harness("claude", { kind: "missing" });
  const checks: Checks = { harnesses: [pi, codex, claude], gitInstall: null, defaultHarness: null };
  assert.deepEqual(blockingRows(null, "pi"), []);
  assert.deepEqual(blockingRows(checks, "pi"), [pi]);
  assert.deepEqual(blockingRows(checks, "claude"), [claude]);
  assert.deepEqual(blockingRows(checks, "codex"), []);
  assert.deepEqual(blockingRows(checks, null), [pi, codex, claude]);
  assert.deepEqual(blockingRows({ ...checks, defaultHarness: "codex" }, null), []);
});
