// Tests for the transcript rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { chatAfter, emptyChat, shownSteps, stepImages, type Step } from "./transcript.ts";

test("chunks of one reply join into one streaming message until it ends", () => {
  let chat = chatAfter(emptyChat, { kind: "message_chunk", text: "Hel" }, 0);
  chat = chatAfter(chat, { kind: "message_chunk", text: "lo" }, 0);
  assert.deepEqual(chat.items, [{ role: "assistant", text: "Hello", streaming: true }]);
  chat = chatAfter(chat, { kind: "message_end" }, 0);
  assert.deepEqual(chat.items, [{ role: "assistant", text: "Hello", streaming: false }]);
});

test("a chunk after a steer starts a new reply", () => {
  let chat = chatAfter(emptyChat, { kind: "message_chunk", text: "One" }, 0);
  chat = chatAfter(chat, { kind: "user_message_chunk", text: "Stop" }, 0);
  chat = chatAfter(chat, { kind: "message_chunk", text: "Two" }, 0);
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
  let chat = chatAfter({ ...emptyChat, agent: "ready" }, { kind: "state", running: true }, 0);
  chat = chatAfter(chat, { kind: "message_chunk", text: "Hi" }, 0);
  chat = chatAfter(chat, { kind: "exited" }, 0);
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
  let chat = chatAfter(emptyChat, { kind: "thought_start" }, 0);
  chat = chatAfter(chat, { kind: "thought_chunk", text: "Hm" }, 0);
  chat = chatAfter(chat, { kind: "tool_start", id: "c", title: "Run", detail: "ls" }, 0);
  chat = chatAfter(chat, { kind: "tool_end", id: "c", outcome: "failed", images: [] }, 0);
  chat = chatAfter(chat, { kind: "message_chunk", text: "Done" }, 0);
  assert.deepEqual(chat.items, [
    { role: "steps", steps: [{ kind: "thought", id: 0, text: "Hm" }, { kind: "call", id: "c", title: "Run", detail: "ls", state: "failed", images: [] }] },
    { role: "assistant", text: "Done", streaming: true },
  ]);
});

test("two thinking blocks stay two thoughts and a stopped call stays stopped", () => {
  let chat = chatAfter(emptyChat, { kind: "thought_start" }, 0);
  chat = chatAfter(chat, { kind: "thought_chunk", text: "A" }, 0);
  chat = chatAfter(chat, { kind: "thought_start" }, 0);
  chat = chatAfter(chat, { kind: "thought_chunk", text: "B" }, 0);
  chat = chatAfter(chat, { kind: "tool_start", id: "c", title: "Run", detail: "sleep 9" }, 0);
  chat = chatAfter(chat, { kind: "tool_end", id: "c", outcome: "stopped", images: [] }, 0);
  chat = chatAfter(chat, { kind: "state", running: false }, 0);
  assert.deepEqual(chat.items, [
    {
      role: "steps",
      steps: [
        { kind: "thought", id: 0, text: "A" },
        { kind: "thought", id: 1, text: "B" },
        { kind: "call", id: "c", title: "Run", detail: "sleep 9", state: "stopped", images: [] },
      ],
    },
  ]);
});

test("a run stamps its start, and the report keeps its time until the status changes", () => {
  let chat = chatAfter({ ...emptyChat, agent: "ready" }, { kind: "state", running: true }, 100);
  chat = chatAfter(chat, { kind: "status", status: { kind: "running" }, last_message: "" }, 200);
  chat = chatAfter(chat, { kind: "status", status: { kind: "running" }, last_message: "Hi" }, 300);
  assert.equal(chat.since, 100);
  assert.equal(chat.report?.since, 200);
  chat = chatAfter(chat, { kind: "status", status: { kind: "done" }, last_message: "Hi" }, 400);
  assert.equal(chat.report?.since, 400);
});

test("a tool call keeps the images its tool returned", () => {
  const image = { data: "x", mime_type: "image/png" };
  let chat = chatAfter(emptyChat, { kind: "tool_start", id: "c", title: "Read", detail: "a.png" }, 0);
  chat = chatAfter(chat, { kind: "tool_end", id: "c", outcome: "done", images: [image] }, 0);
  assert.deepEqual(chat.items, [{ role: "steps", steps: [{ kind: "call", id: "c", title: "Read", detail: "a.png", state: "done", images: [image] }] }]);
});

test("a group's images are its calls' images in call order", () => {
  const a = { data: "a", mime_type: "image/png" };
  const b = { data: "b", mime_type: "image/png" };
  const c = { data: "c", mime_type: "image/png" };
  const steps: Step[] = [
    { kind: "call", id: "1", title: "Read", detail: "a", state: "done", images: [a, b] },
    { kind: "thought", id: 0, text: "Hm" },
    { kind: "call", id: "2", title: "Run", detail: "ls", state: "done" },
    { kind: "call", id: "3", title: "Read", detail: "c", state: "done", images: [c] },
  ];
  assert.deepEqual(stepImages(steps), [a, b, c]);
});
