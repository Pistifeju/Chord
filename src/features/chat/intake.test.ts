import assert from "node:assert/strict";
import { test } from "node:test";
import { intakes } from "./intake.ts";
import { emptyChat } from "./transcript.ts";

test("a new start drops the events of the older one, and ending the pane ends its wait", () => {
  const intake = intakes();
  let waits = 0;
  const first = intake.begin("p", () => waits++);
  const second = intake.begin("p", () => waits++);
  assert.equal(intake.fed("p", first, emptyChat, { kind: "notice", text: "old" }, 0), null);
  assert.equal(intake.fed("p", second, emptyChat, { kind: "notice", text: "new" }, 0)?.chat.notice, "new");
  intake.end("p");
  assert.deepEqual([waits, intake.has("p")], [1, false]);
});

test("an export is a notice outside the transcript", () => {
  const intake = intakes();
  const start = intake.begin("p", () => {});
  const fed = intake.fed("p", start, emptyChat, { kind: "exported", path: "/s.html" }, 0);
  assert.deepEqual(fed, { chat: emptyChat, effect: { kind: "notify", text: "Exported to /s.html" } });
});
