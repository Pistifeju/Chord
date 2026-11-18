// Tests for the browser address rules.
import { test } from "node:test";
import assert from "node:assert/strict";
import { pageName, webAddress } from "./address.ts";

test("typed text opens a web address, a bare host over https and a local host over http", () => {
  assert.equal(webAddress(" https://example.com/a "), "https://example.com/a");
  assert.equal(webAddress("example.com"), "https://example.com/");
  assert.equal(webAddress("localhost:3000/app"), "http://localhost:3000/app");
  assert.equal(webAddress("two words"), null);
  assert.equal(webAddress("readme"), null);
  assert.equal(webAddress("file:///etc/passwd"), null);
});

test("a tab shows the page title, else its host", () => {
  assert.equal(pageName("https://example.com/a", " Example "), "Example");
  assert.equal(pageName("https://example.com/a", ""), "example.com");
});
