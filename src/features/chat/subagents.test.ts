// Tests for the summary line of several subagent runs.
import assert from "node:assert/strict";
import { test } from "node:test";
import type { SubagentNode } from "../../ipc/bindings";
import { subagentSummary } from "./subagents.ts";

const node = (id: string, depth: number, state: SubagentNode["state"], started_at: number | null = null, ended_at: number | null = null): SubagentNode => ({
  id,
  depth,
  name: id,
  state,
  tool: null,
  started_at,
  ended_at,
});

test("a chain counts its steps, not the run that holds them, and the time runs from the first start to now", () => {
  const nodes = [node("chain", 0, "running", 1000), node("scout", 1, "done"), node("build", 1, "running"), node("worker", 0, "done", 2000, 9000)];
  assert.deepEqual(subagentSummary(nodes, 31000), { text: "3 subagents · 30s", mark: "running" });
});

test("settled runs take their time to the last end, and a failed one marks the summary", () => {
  const nodes = [node("a", 0, "done", 1000, 5000), node("b", 0, "error", 2000, 66000)];
  assert.deepEqual(subagentSummary(nodes, 99000), { text: "2 subagents · 1m 5s", mark: "error" });
});

test("one agent gets no summary, also inside a run", () => {
  assert.equal(subagentSummary([node("a", 0, "running", 1000)], 2000), null);
  assert.equal(subagentSummary([node("run", 0, "running", 1000), node("a", 1, "running")], 2000), null);
});
