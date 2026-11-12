// Tests for the Markdown rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { blocksOf, inlineFrom } from "./markdown.ts";

const text = (t: string) => ({ kind: "text", text: t });

test("a nested list with tasks keeps its levels", () => {
  const [list] = blocksOf("- [x] done\n- [ ] open\n  1. one\n  2. two");
  assert.deepEqual(list, {
    kind: "list",
    ordered: false,
    start: 1,
    items: [
      { checked: true, children: [{ kind: "paragraph", children: [text("done")] }] },
      {
        checked: false,
        children: [
          { kind: "paragraph", children: [text("open")] },
          {
            kind: "list",
            ordered: true,
            start: 1,
            items: [
              { checked: null, children: [{ kind: "paragraph", children: [text("one")] }] },
              { checked: null, children: [{ kind: "paragraph", children: [text("two")] }] },
            ],
          },
        ],
      },
    ],
  });
});

test("a fence keeps its text, language and blank lines, and ends the paragraph above", () => {
  assert.deepEqual(blocksOf("Run:\n```ts\nlet a = 1;\n\n# not a heading\n```\nAfter"), [
    { kind: "paragraph", children: [text("Run:")] },
    { kind: "code", lang: "ts", text: "let a = 1;\n\n# not a heading" },
    { kind: "paragraph", children: [text("After")] },
  ]);
});

test("a table takes its alignment from the delimiter row and pads short rows", () => {
  const [table] = blocksOf("| a | b |\n|:--|--:|\n| 1 |\n| 2 | 3 |");
  assert.deepEqual(table, {
    kind: "table",
    head: [[text("a")], [text("b")]],
    align: ["left", "right"],
    rows: [[[text("1")], []], [[text("2")], [text("3")]]],
  });
});

test("a quote holds blocks and a heading takes its level", () => {
  assert.deepEqual(blocksOf("## Title\n> a\n> - b"), [
    { kind: "heading", level: 2, children: [text("Title")] },
    {
      kind: "quote",
      children: [
        { kind: "paragraph", children: [text("a")] },
        { kind: "list", ordered: false, start: 1, items: [{ checked: null, children: [{ kind: "paragraph", children: [text("b")] }] }] },
      ],
    },
  ]);
});

test("inline spans nest, and code and escapes stay literal", () => {
  assert.deepEqual(inlineFrom("**a *b*** `*c*` \\*d\\* snake_case_name"), [
    { kind: "strong", children: [text("a "), { kind: "em", children: [text("b")] }] },
    text(" "),
    { kind: "code", text: "*c*" },
    text(" *d* snake_case_name"),
  ]);
});

test("only web and mail links become links; a bare URL drops its closing punctuation", () => {
  assert.deepEqual(inlineFrom("[a](https://x.dev/p) [b](javascript:alert(1)) see https://y.dev/q)."), [
    { kind: "link", href: "https://x.dev/p", children: [text("a")] },
    text(" "),
    text("b"),
    text(" see "),
    { kind: "link", href: "https://y.dev/q", children: [text("https://y.dev/q")] },
    text(")."),
  ]);
});
