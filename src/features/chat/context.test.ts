// Tests for the context meter rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { contextMeter } from "./context.ts";
import { emptyChat } from "./transcript.ts";

const chat = (tokens: number | null) => ({ ...emptyChat, capabilities: ["context" as const], context: { tokens, window: 200000, estimated: false } });

test("the meter shows the fill of the window", () => {
  const meter = contextMeter(chat(60000));
  assert.equal(meter?.percent, 30);
  assert.equal(meter?.level, "calm");
});

test("the meter warns from 50 percent and turns critical from 75", () => {
  assert.equal(contextMeter(chat(98000))?.level, "calm");
  assert.equal(contextMeter(chat(100000))?.level, "warning");
  assert.equal(contextMeter(chat(148000))?.level, "warning");
  assert.equal(contextMeter(chat(150000))?.level, "critical");
});

test("a window without a token count shows unknown", () => {
  assert.equal(contextMeter(chat(null))?.percent, null);
});

test("the meter hides without the capability or the window", () => {
  assert.equal(contextMeter({ ...chat(1), capabilities: ["chat"] }), null);
  assert.equal(contextMeter({ ...chat(1), context: null }), null);
  assert.equal(contextMeter({ ...chat(1), context: { tokens: 1, window: null, estimated: false } }), null);
});

test("an empty session shows 0 percent before the agent tells the window", () => {
  assert.equal(contextMeter({ ...chat(0), context: { tokens: 0, window: null, estimated: false } })?.percent, 0);
});
