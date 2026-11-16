// Tests for the slash menu rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { commandTitle, markedTitle, slashMenu } from "./slash.ts";

const commands = [
  { name: "compact", description: "", kind: "skill" as const, origin: null },
  { name: "skill:deploy", description: "Ship it", kind: "skill" as const, origin: null },
  { name: "deploy-docs", description: "", kind: "skill" as const, origin: null },
  { name: "undeploy", description: "", kind: "skill" as const, origin: null },
];

test("names that start with the typed part come first, then the others by where it sits", () => {
  assert.deepEqual(
    slashMenu(commands, "/dep", undefined).map((command) => command.name),
    ["deploy-docs", "undeploy", "skill:deploy"],
  );
  assert.deepEqual(
    slashMenu(commands, "/", undefined).map((command) => command.name),
    ["compact", "skill:deploy", "deploy-docs", "undeploy"],
  );
});

test("a name equal to the typed part comes before longer names that start with it", () => {
  assert.deepEqual(
    slashMenu([{ name: "deploy-docs", description: "", kind: "skill" as const, origin: null }, { name: "deploy", description: "", kind: "skill" as const, origin: null }], "/deploy", undefined).map((command) => command.name),
    ["deploy", "deploy-docs"],
  );
});

test("the menu stays shut for text that is not a bare command name or was closed on this draft", () => {
  assert.deepEqual(slashMenu(commands, "hi /dep", undefined), []);
  assert.deepEqual(slashMenu(commands, "/deploy-docs now", undefined), []);
  assert.deepEqual(slashMenu(commands, "/dep", "/dep"), []);
  assert.equal(slashMenu(commands, "/dep", "/de").length, 3);
});

test("an extension command named like a skill gives its row to the skill", () => {
  assert.deepEqual(
    slashMenu([{ name: "review", description: "", kind: "extension" as const, origin: "pkg" }, { name: "skill:review", description: "", kind: "skill" as const, origin: "pkg" }, { name: "review-loop", description: "", kind: "extension" as const, origin: "pkg" }], "/review", undefined).map((command) => command.name),
    ["review-loop", "skill:review"],
  );
});

test("a command name becomes a title", () => {
  assert.equal(commandTitle("skill:improve-codebase-architecture"), "Improve Codebase Architecture");
  assert.equal(commandTitle("fix_tests"), "Fix Tests");
  assert.equal(commandTitle("mcp:status"), "Mcp Status");
});

test("a title splits around the typed part from the start of a word, ignoring case and separators", () => {
  assert.deepEqual(markedTitle("Improve Codebase", "imp"), { before: "", typed: "Imp", after: "rove Codebase" });
  assert.deepEqual(markedTitle("Improve Codebase", "improve-co"), { before: "", typed: "Improve Co", after: "debase" });
  assert.deepEqual(markedTitle("Improve Codebase", "code"), { before: "Improve ", typed: "Code", after: "base" });
  assert.deepEqual(markedTitle("Undeploy", "dep"), { before: "Undeploy", typed: "", after: "" });
  assert.deepEqual(markedTitle("Compact", ""), { before: "Compact", typed: "", after: "" });
});
