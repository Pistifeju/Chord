// Tests for the transcript rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { callTime, chatAfter, chatFed, emptyChat, queuedImages, stepImages, stepsChanges, stepsSummary, withSteerImages, type Message, type Step } from "./transcript.ts";

test("chunks of one reply join into one streaming message until it ends", () => {
  let chat = chatAfter(emptyChat, { kind: "message_chunk", text: "Hel" }, 0);
  chat = chatAfter(chat, { kind: "message_chunk", text: "lo" }, 0);
  assert.deepEqual(chat.items, [{ role: "assistant", text: "Hello", streaming: true }]);
  chat = chatAfter(chat, { kind: "message_end", failed: false, at: 1000 }, 0);
  assert.deepEqual(chat.items, [{ role: "assistant", text: "Hello", streaming: false, at: 1000 }]);
});

test("a chunk after a steer starts a new reply", () => {
  let chat = chatAfter(emptyChat, { kind: "message_chunk", text: "One" }, 0);
  chat = chatAfter(chat, { kind: "user_message_chunk", text: "Stop", images: [] }, 0);
  chat = chatAfter(chat, { kind: "message_chunk", text: "Two" }, 0);
  assert.deepEqual(
    (chat.items as Message[]).map((m) => [m.role, m.text]),
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
  assert.equal((chat.items[0] as Message).streaming, false);
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
  chat = chatAfter(chat, { kind: "status", status: { kind: "running" }, last_message: "", changed: true }, 200);
  chat = chatAfter(chat, { kind: "status", status: { kind: "running" }, last_message: "Hi", changed: false }, 300);
  assert.equal(chat.since, 100);
  assert.equal(chat.report?.since, 200);
  chat = chatAfter(chat, { kind: "status", status: { kind: "done" }, last_message: "Hi", changed: true }, 400);
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

test("a dialog stays open until Rust closes it", () => {
  const ask = { kind: "confirm", message: "Sure?" } as const;
  let chat = chatAfter(emptyChat, { kind: "dialog", id: "u1", title: "Clear", ask }, 0);
  assert.deepEqual(chat.dialog, { id: "u1", title: "Clear", ask });
  chat = chatAfter(chat, { kind: "queue", steering: [] }, 0);
  assert.equal(chat.dialog?.id, "u1");
  chat = chatAfter(chat, { kind: "dialog_closed" }, 0);
  assert.equal(chat.dialog, null);
});

test("a picker menu stays open past a dialog close, until the agent exits", () => {
  const ask = { kind: "menu" as const, options: ["a", "b"], current: "a", filter: false };
  let chat = chatAfter(emptyChat, { kind: "dialog", id: "m1", title: "Model", ask }, 0);
  chat = chatAfter(chat, { kind: "dialog_closed" }, 0);
  assert.equal(chat.dialog?.id, "m1");
  chat = chatAfter(chat, { kind: "exited", code: 0 }, 0);
  assert.equal(chat.dialog, null);
});

test("a history start empties the transcript and the status, then the saved messages fill it", () => {
  let chat = chatAfter(emptyChat, { kind: "user_message_chunk", text: "Old", images: [] }, 0);
  chat = chatAfter(chat, { kind: "status", status: { kind: "done" }, last_message: "Old reply", changed: true }, 5);
  chat = chatAfter(chat, { kind: "history_start" }, 9);
  assert.deepEqual([chat.items, chat.report], [[], null]);
  chat = chatAfter(chat, { kind: "user_message_chunk", text: "Saved", images: [], at: 7 }, 9);
  assert.deepEqual(chat.items, [{ role: "user", text: "Saved", streaming: false, images: [], at: 7 }]);
});

test("a session's history changes the chat once, at its end", () => {
  let fed = chatFed(emptyChat, null, { kind: "history_start" }, 1);
  fed = chatFed(fed.chat, fed.replay, { kind: "user_message_chunk", text: "Hi", images: [], at: 5 }, 2);
  assert.deepEqual([fed.chat.items, fed.chat.loading], [[], true]);
  fed = chatFed(fed.chat, fed.replay, { kind: "history_end" }, 3);
  assert.equal(fed.replay, null);
  assert.deepEqual(fed.chat.items, [{ role: "user", text: "Hi", streaming: false, images: [], at: 5 }]);
  assert.deepEqual([fed.chat.historyLength, fed.chat.loading], [1, false]);
});

test("a second history start keeps what the replay held before it", () => {
  let fed = chatFed(emptyChat, null, { kind: "history_start" }, 1);
  fed = chatFed(fed.chat, fed.replay, { kind: "commands", commands: [{ name: "new", description: "", kind: "session", origin: null }] }, 2);
  fed = chatFed(fed.chat, fed.replay, { kind: "history_start" }, 3);
  fed = chatFed(fed.chat, fed.replay, { kind: "history_end" }, 4);
  assert.deepEqual(fed.chat.commands, [{ name: "new", description: "", kind: "session", origin: null }]);
});

const call = (title: string, detail: string, state: "done" | "failed" = "done", changes?: { added: number; removed: number }): Step => ({ kind: "call", id: `${title}-${detail}`, title, detail, state, ...(changes && { changes }) });

test("the step summary counts calls by kind and edited files once each", () => {
  const steps = [call("Search", "a"), call("Search", "b"), call("Edit", "x.ts"), call("Edit", "x.ts"), call("Write", "y.ts"), call("Run", "ls"), call("Github list", "z")];
  assert.equal(stepsSummary(steps), "2 searches, edited 2 files, ran 1 command, 1 tool call");
  assert.equal(stepsSummary([{ kind: "thought", id: 0, text: "" }, call("Read", "a")]), "1 thought, read 1 file");
});

test("the step changes sum the finished calls' lines", () => {
  const steps = [call("Edit", "x.ts", "done", { added: 3, removed: 1 }), call("Write", "y.ts", "done", { added: 40, removed: 0 }), call("Edit", "z.ts", "failed", { added: 9, removed: 9 })];
  assert.deepEqual(stepsChanges(steps), { added: 43, removed: 1 });
  assert.equal(stepsChanges([call("Run", "ls")]), null);
});

test("a run time of a minute or more reads in minutes", () => {
  const slow = (ms: number): Step => ({ kind: "call", id: "c", title: "Run", detail: "ls", state: "done", images: [], ms });
  assert.equal(callTime(slow(6600)), "6.6 s");
  assert.equal(callTime(slow(556_000)), "9m 16s");
});

test("a steer's images show in the queue until the agent takes the steer in", () => {
  const image = { data: "x", mime_type: "image/png" };
  let chat = withSteerImages({ ...emptyChat, running: true }, "look", [image]);
  chat = chatAfter(chat, { kind: "queue", steering: ["look"] }, 0);
  assert.deepEqual(queuedImages(chat.steerImages, "look"), [image]);
  chat = chatAfter(chat, { kind: "user_message_chunk", text: "look", images: [image] }, 0);
  assert.deepEqual(chat.steerImages, []);
});
