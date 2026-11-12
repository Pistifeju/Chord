// Tests for the transcript rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { chatAfter, emptyChat, shownSteps, type Step } from "./transcript.ts";

test("chunks of one reply join into one streaming message until it ends", () => {
  let chat = chatAfter(emptyChat, { kind: "message_chunk", text: "Hel" });
  chat = chatAfter(chat, { kind: "message_chunk", text: "lo" });
  assert.deepEqual(chat.items, [{ role: "assistant", text: "Hello", streaming: true }]);
  chat = chatAfter(chat, { kind: "message_end" });
  assert.deepEqual(chat.items, [{ role: "assistant", text: "Hello", streaming: false }]);
});

test("a chunk after a steer starts a new reply", () => {
  let chat = chatAfter(emptyChat, { kind: "message_chunk", text: "One" });
  chat = chatAfter(chat, { kind: "user_message_chunk", text: "Stop" });
  chat = chatAfter(chat, { kind: "message_chunk", text: "Two" });
  assert.deepEqual(
    chat.items.map((m) => [m.role, m.text]),
    [
      ["assistant", "One"],
      ["user", "Stop"],
      ["assistant", "Two"],
    ],
  );
});

test("an exit stops the run and the stream", () => {
  let chat = chatAfter({ ...emptyChat, agent: "ready" }, { kind: "state", running: true });
  chat = chatAfter(chat, { kind: "message_chunk", text: "Hi" });
  chat = chatAfter(chat, { kind: "exited" });
  assert.equal(chat.agent, "idle");
  assert.equal(chat.running, false);
  assert.equal(chat.items[0].streaming, false);
});

test("a running group shows its last thought and last three calls", () => {
  const call = (id: string): Step => ({ kind: "call", id, title: "Read", detail: id, state: "done" });
  const steps: Step[] = [{ kind: "thought", id: 0, text: "a" }, call("1"), { kind: "thought", id: 2, text: "b" }, call("2"), call("3"), call("4")];
  assert.deepEqual(shownSteps(steps, true, false), [
    { kind: "thought", id: 2, text: "b" },
    call("2"),
    call("3"),
    call("4"),
  ]);
  assert.deepEqual(shownSteps(steps, false, false), []);
  assert.deepEqual(shownSteps(steps, false, true), steps);
});

test("thoughts and calls fold into one group that the next reply closes", () => {
  let chat = chatAfter(emptyChat, { kind: "thought_start" });
  chat = chatAfter(chat, { kind: "thought_chunk", text: "Hm" });
  chat = chatAfter(chat, { kind: "tool_start", id: "c", title: "Run", detail: "ls" });
  chat = chatAfter(chat, { kind: "tool_end", id: "c", outcome: "failed" });
  chat = chatAfter(chat, { kind: "message_chunk", text: "Done" });
  assert.deepEqual(chat.items, [
    { role: "steps", steps: [{ kind: "thought", id: 0, text: "Hm" }, { kind: "call", id: "c", title: "Run", detail: "ls", state: "failed" }] },
    { role: "assistant", text: "Done", streaming: true },
  ]);
});

test("two thinking blocks stay two thoughts and a stopped call stays stopped", () => {
  let chat = chatAfter(emptyChat, { kind: "thought_start" });
  chat = chatAfter(chat, { kind: "thought_chunk", text: "A" });
  chat = chatAfter(chat, { kind: "thought_start" });
  chat = chatAfter(chat, { kind: "thought_chunk", text: "B" });
  chat = chatAfter(chat, { kind: "tool_start", id: "c", title: "Run", detail: "sleep 9" });
  chat = chatAfter(chat, { kind: "tool_end", id: "c", outcome: "stopped" });
  chat = chatAfter(chat, { kind: "state", running: false });
  assert.deepEqual(chat.items, [
    {
      role: "steps",
      steps: [
        { kind: "thought", id: 0, text: "A" },
        { kind: "thought", id: 1, text: "B" },
        { kind: "call", id: "c", title: "Run", detail: "sleep 9", state: "stopped" },
      ],
    },
  ]);
});
