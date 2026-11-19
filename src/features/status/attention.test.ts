// Tests for the attention queue and the next agent needing you.
import assert from "node:assert/strict";
import { test } from "node:test";
import { attentionQueue, nextAttention, type AgentPane } from "./attention.ts";
import type { StatusReport } from "./status.ts";

const chat = (kind: "running" | "waiting" | "done" | "error", since: number) => ({ report: { status: { kind }, lastMessage: "", since } as StatusReport });
const all: AgentPane[] = ["a", "b", "c", "d", "e", "f"].map((pane) => ({ worktree: "w", pane }));
const chats = { a: chat("done", 1), b: chat("waiting", 5), c: chat("error", 2), d: chat("waiting", 3), e: chat("running", 0), f: chat("done", 0) };

test("the queue holds waits first, then unseen failures and dones, the longest wait first", () => {
  assert.deepEqual(attentionQueue(all, chats, ["a", "c", "e"]).map((entry) => entry.pane), ["d", "b", "c", "a"]);
});

test("the next agent follows the current pane and wraps; a lone current pane has no next", () => {
  const queue = attentionQueue(all, chats, ["a", "c"]);
  assert.equal(nextAttention(queue, "e")?.pane, "d");
  assert.equal(nextAttention(queue, "b")?.pane, "c");
  assert.equal(nextAttention(queue, "a")?.pane, "d");
  assert.equal(nextAttention(queue.slice(0, 1), "d"), null);
  assert.equal(nextAttention([], null), null);
});
