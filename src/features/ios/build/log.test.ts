// Tests for the build log rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { appendToLog, emptyLog, LOG_LINES, logLines, matchesIn, segmentsOf } from "./log.ts";

test("text joins into lines and the unfinished line waits for its break", () => {
  const first = appendToLog(emptyLog, "one\ntw");
  assert.deepEqual(first, { lines: ["one"], partial: "tw" });
  const second = appendToLog(first, "o\nthree\n");
  assert.deepEqual(second, { lines: ["one", "two", "three"], partial: "" });
  assert.deepEqual(logLines(appendToLog(second, "four")), ["one", "two", "three", "four"]);
});

test("the log drops its oldest lines only once 1,000 pile up past the cap, and keeps the newest 10,000", () => {
  const lines = (count: number) => Array.from({ length: count }, (_, n) => `line ${n}`).join("\n") + "\n";
  const within = appendToLog(emptyLog, lines(LOG_LINES + 1_000));
  assert.equal(within.lines.length, 11_000);
  assert.equal(within.lines[0], "line 0");
  const past = appendToLog(emptyLog, lines(LOG_LINES + 1_005));
  assert.equal(past.lines.length, 10_000);
  assert.equal(past.lines[0], "line 1005");
  assert.equal(past.lines[9_999], "line 11004");
});

test("a query matches every occurrence in any case", () => {
  assert.deepEqual(matchesIn(["Error: error", "none", "ERROR"], "error"), [
    { line: 0, start: 0 },
    { line: 0, start: 7 },
    { line: 2, start: 0 },
  ]);
  assert.deepEqual(matchesIn(["abc"], ""), []);
});

test("a line is cut at its matches", () => {
  assert.deepEqual(segmentsOf("a Err b err", "err"), [
    { text: "a ", match: false, start: 0 },
    { text: "Err", match: true, start: 2 },
    { text: " b ", match: false, start: 5 },
    { text: "err", match: true, start: 8 },
  ]);
  assert.deepEqual(segmentsOf("plain", "x"), [{ text: "plain", match: false, start: 0 }]);
});
