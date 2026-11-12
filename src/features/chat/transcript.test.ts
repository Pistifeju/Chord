// Tests for the transcript rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { chatAfter, emptyChat } from "./transcript.ts";

test("chunks of one reply join into one streaming message until it ends", () => {
  let chat = chatAfter(emptyChat, { kind: "message_chunk", text: "Hel" });
  chat = chatAfter(chat, { kind: "message_chunk", text: "lo" });
  assert.deepEqual(chat.messages, [{ role: "assistant", text: "Hello", streaming: true }]);
  chat = chatAfter(chat, { kind: "message_end" });
  assert.deepEqual(chat.messages, [{ role: "assistant", text: "Hello", streaming: false }]);
});

test("a chunk after a steer starts a new reply", () => {
  let chat = chatAfter(emptyChat, { kind: "message_chunk", text: "One" });
  chat = chatAfter(chat, { kind: "user_message_chunk", text: "Stop" });
  chat = chatAfter(chat, { kind: "message_chunk", text: "Two" });
  assert.deepEqual(
    chat.messages.map((m) => [m.role, m.text]),
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
  assert.equal(chat.messages[0].streaming, false);
});
