// Tests for the agent picker's level rules and where another agent's pick starts.
import assert from "node:assert/strict";
import { test } from "node:test";
import { levelSlots, shownLevel, switchesInPlace } from "./agentPicker.ts";
import { emptyChat } from "./transcript.ts";

const sonnet = { id: "sonnet", name: "Sonnet", provider: null, levels: ["low", "medium", "high"], default_level: "medium" };

test("the bar marks the picked level the model has, else the running level the model has, else its default", () => {
  const running = { id: "opus", name: "Opus", thinking: "high" };
  assert.equal(shownLevel(sonnet, running, "low"), "low");
  assert.equal(shownLevel(sonnet, running, "xhigh"), "high");
  assert.equal(shownLevel(sonnet, running, null), "high");
  assert.equal(shownLevel(sonnet, { ...running, thinking: "xhigh" }, null), "medium");
  assert.equal(shownLevel({ ...sonnet, levels: [] }, running, "low"), null);
});

test("the bar's segments hold every model's levels in order, a level one model lacks in its place", () => {
  const models = [
    { ...sonnet, levels: ["off", "low", "high"] },
    { ...sonnet, levels: ["off", "minimal", "low", "medium", "high", "xhigh"] },
    { ...sonnet, levels: [] },
  ];
  assert.deepEqual(levelSlots(models), ["off", "minimal", "low", "medium", "high", "xhigh"]);
});

test("another agent's pick switches in place only on an empty chat whose history has loaded and holds no message", () => {
  assert.equal(switchesInPlace(emptyChat), true);
  assert.equal(switchesInPlace({ ...emptyChat, loading: true }), false);
  assert.equal(switchesInPlace({ ...emptyChat, held: "hi" }), false);
});
