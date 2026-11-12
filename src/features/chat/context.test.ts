// Tests for the context meter rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { contextMeter } from "./context.ts";
import { emptyChat } from "./transcript.ts";

const chat = (tokens: number | null) => ({ ...emptyChat, capabilities: ["context" as const], context: { tokens, window: 200000 } });

test("the meter shows the fill of the window", () => {
  const meter = contextMeter(chat(60000));
  assert.equal(meter?.percent, 30);
  assert.equal(meter?.warning, false);
});

test("the meter warns from 80 percent", () => {
  assert.equal(contextMeter(chat(160000))?.warning, true);
  assert.equal(contextMeter(chat(150000))?.warning, false);
});

test("a window without a token count shows unknown", () => {
  assert.equal(contextMeter(chat(null))?.percent, null);
});

test("the meter hides without the capability or the window", () => {
  assert.equal(contextMeter({ ...chat(1), capabilities: ["chat"] }), null);
  assert.equal(contextMeter({ ...chat(1), context: null }), null);
});
