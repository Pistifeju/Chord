// Tests for the running simulator and build data size rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { hasOtherSimulators, sizeText } from "./simulators.ts";

test("Shut Down Others has work only for simulators beyond the open worktree's", () => {
  const own = { worktree: "a", model: "m", open: true };
  assert.equal(hasOtherSimulators([own]), false);
  assert.equal(hasOtherSimulators([own, { worktree: "b", model: "m", open: false }]), true);
});

test("sizes use decimal units", () => {
  assert.equal(sizeText(12_449_000_000), "12.4 GB");
  assert.equal(sizeText(512_300_000), "512 MB");
  assert.equal(sizeText(0), "0 KB");
});
