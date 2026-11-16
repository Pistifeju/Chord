// Tests for the Appearance menu's rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { namedCodes } from "./appearance.ts";

test("codes sort by their names, and a code without a name keeps itself", () => {
  const names: Record<string, string> = { de: "German", en: "English" };
  assert.deepEqual(namedCodes(["en", "xx", "de"], (code) => names[code]), [
    { code: "en", name: "English" },
    { code: "de", name: "German" },
    { code: "xx", name: "xx" },
  ]);
});
