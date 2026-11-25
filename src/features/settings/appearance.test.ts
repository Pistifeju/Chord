import { test } from "node:test";
import assert from "node:assert/strict";
import { shownTheme, steppedTextPercent } from "./appearance.ts";

test("text size moves by 10 percent, stops at 80 and 150, and resets to 100", () => {
  assert.equal(steppedTextPercent(100, "larger"), 110);
  assert.equal(steppedTextPercent(150, "larger"), 150);
  assert.equal(steppedTextPercent(80, "smaller"), 80);
  assert.equal(steppedTextPercent(130, "reset"), 100);
});

test("the system theme shows what the system shows; light and dark ignore it", () => {
  assert.equal(shownTheme("system", true), "dark");
  assert.equal(shownTheme("system", false), "light");
  assert.equal(shownTheme("light", true), "light");
  assert.equal(shownTheme("dark", false), "dark");
});
