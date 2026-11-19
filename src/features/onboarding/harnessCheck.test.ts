// Tests for the command each harness row needs.
import { test } from "node:test";
import assert from "node:assert/strict";
import type { HarnessCheck, Install } from "../../ipc/bindings";
import { neededCommand } from "./harnessCheck.ts";

const harness = (name: string, installState: Install, starts = true): HarnessCheck => ({
  name,
  title: name,
  starts,
  install: `install ${name}`,
  signIn: `login ${name}`,
  upgrade: `update ${name}`,
  installState,
  takesPick: false,
});

test("a row needs the install, sign-in or upgrade command, and none when ready", () => {
  assert.equal(neededCommand(harness("pi", { kind: "missing" })), "install pi");
  assert.equal(neededCommand(harness("pi", { kind: "signed_out", version: "1.0.0" })), "login pi");
  assert.equal(neededCommand(harness("pi", { kind: "outdated", version: "1.0.0", oldest: "2.0.0" })), "update pi");
  assert.equal(neededCommand(harness("pi", { kind: "ready", version: "1.0.0" })), null);
});
