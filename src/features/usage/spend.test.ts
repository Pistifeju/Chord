import assert from "node:assert/strict";
import { test } from "node:test";
import { detail, list } from "./spend.ts";

const HOUR = 3600000;
const now = 100 * HOUR;
const row = (harness: string, model: string, input: number, cost: number | null) => ({ hour: now - HOUR, harness, model, input, cacheRead: 0, cacheWrite: 0, output: 0, cost, saved: null });
const spend = {
  rows: [row("pi", "p/a", 600, 3), row("pi", "p/b", 100, 1), row("codex", "c", 300, null)],
  sessions: { pi: 4, codex: 1 },
};

test("the list ranks harnesses and their models by the shown value, each with its share of the total", () => {
  assert.deepEqual(
    list(spend, "tokens").map((r) => [r.key, r.value, r.share]),
    [
      ["all", 1000, 1],
      ["pi", 700, 0.7],
      ["pi/p/a", 600, 0.6],
      ["pi/p/b", 100, 0.1],
      ["codex", 300, 0.3],
      ["codex/c", 300, 0.3],
    ],
  );
  assert.deepEqual(
    list(spend, "cost").map((r) => [r.key, r.value, r.share]),
    [
      ["all", 4, 1],
      ["pi", 4, 1],
      ["pi/p/a", 3, 0.75],
      ["pi/p/b", 1, 0.25],
      ["codex", null, null],
      ["codex/c", null, null],
    ],
  );
});

test("a model's detail sums only that model and has no session count", () => {
  const picked = detail(spend, "24h", now, "cost", { harness: "pi", model: "p/b" });
  assert.equal(picked.total, 1);
  assert.equal(picked.figures.processed, 100);
  assert.equal(picked.sessions, null);
  assert.equal(detail(spend, "24h", now, "cost", { harness: "codex" }).total, null);
  assert.equal(detail(spend, "24h", now, "tokens", {}).sessions, 5);
});
