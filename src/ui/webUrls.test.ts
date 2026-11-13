// Tests for bare web URLs in text.
import assert from "node:assert/strict";
import { test } from "node:test";
import { webUrlsIn } from "./webUrls.ts";

test("quotes end a URL and each URL keeps its offset", () => {
  assert.deepEqual(webUrlsIn('["https://a.dev/x","https://b.dev"].'), [
    { url: "https://a.dev/x", start: 2 },
    { url: "https://b.dev", start: 20 },
  ]);
});
