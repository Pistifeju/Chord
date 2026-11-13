// Rules for the agent's Markdown: text in, blocks and inline spans out. Raw HTML is never read, so it stays text.
import { webUrlAt } from "../../ui/webUrls.ts";

export type Inline =
  | { kind: "text"; text: string }
  | { kind: "code"; text: string }
  | { kind: "strong" | "em" | "del"; children: Inline[] }
  | { kind: "link"; href: string; children: Inline[] }
  | { kind: "break" };

export type ListItem = { checked: boolean | null; children: Block[] };

export type Align = "left" | "center" | "right" | null;

export type Block =
  | { kind: "heading"; level: number; children: Inline[] }
  | { kind: "paragraph"; children: Inline[] }
  | { kind: "code"; lang: string; text: string }
  | { kind: "quote"; children: Block[] }
  | { kind: "list"; ordered: boolean; start: number; items: ListItem[] }
  | { kind: "table"; head: Inline[][]; align: Align[]; rows: Inline[][][] }
  | { kind: "rule" };

const FENCE = /^ {0,3}(`{3,}|~{3,})\s*([^\s`]*)/;
const HEADING = /^ {0,3}(#{1,6})(?:\s+(.*?))?(?:\s+#+)?\s*$/;
const RULE = /^ {0,3}([-*_])(?:\s*\1){2,}\s*$/;
const QUOTE = /^ {0,3}> ?/;
const ITEM = /^( *)([-*+]|\d{1,9}[.)])(?:\s+(.*))?$/;
const DELIMITER_ROW = /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/;
const TASK = /^\[([ xX])\]\s+/;
const PUNCTUATION = "\\`*_{}[]()#+-.!|~<>";

const isBlank = (line: string) => line.trim() === "";
const indentOf = (line: string) => line.length - line.trimStart().length;
const isOrdered = (marker: string) => /\d/.test(marker);

// A line that ends the paragraph above it. An ordered item interrupts only when it starts at 1.
function startsBlock(line: string): boolean {
  if (FENCE.test(line) || HEADING.test(line) || RULE.test(line) || QUOTE.test(line)) return true;
  const item = ITEM.exec(line);
  return !!item && item[3] !== undefined && (!isOrdered(item[2]) || parseInt(item[2], 10) === 1);
}

function splitRow(line: string): string[] {
  const cells: string[] = [];
  let cell = "";
  for (let i = 0; i < line.length; i++) {
    if (line[i] === "\\" && line[i + 1] === "|") {
      cell += "|";
      i++;
    } else if (line[i] === "|") {
      cells.push(cell);
      cell = "";
    } else cell += line[i];
  }
  cells.push(cell);
  const trimmed = line.trim();
  if (trimmed.startsWith("|")) cells.shift();
  if (trimmed.endsWith("|") && !trimmed.endsWith("\\|")) cells.pop();
  return cells.map((c) => c.trim());
}

const alignOf = (cell: string): Align => {
  const left = cell.startsWith(":");
  const right = cell.endsWith(":");
  return left && right ? "center" : right ? "right" : left ? "left" : null;
};

const closesFence = (line: string, fence: string) => {
  const t = line.trim();
  return t.length >= fence.length && t === fence[0].repeat(t.length);
};

function listFrom(lines: string[], start: number): [Block, number] {
  const first = ITEM.exec(lines[start])!;
  const ordered = isOrdered(first[2]);
  const items: ListItem[] = [];
  let i = start;
  let contentIndent = 0;
  while (i < lines.length) {
    const m = ITEM.exec(lines[i]);
    if (!m || (items.length > 0 && (m[1].length >= contentIndent || isOrdered(m[2]) !== ordered))) break;
    contentIndent = m[1].length + m[2].length + 1;
    const body = [m[3] ?? ""];
    i++;
    while (i < lines.length) {
      const line = lines[i];
      if (isBlank(line)) {
        let k = i;
        while (k < lines.length && isBlank(lines[k])) k++;
        const next = lines[k];
        if (next === undefined || indentOf(next) < contentIndent) break;
        body.push("");
      } else if (indentOf(line) >= contentIndent) body.push(line.slice(contentIndent));
      else if (!isBlank(body[body.length - 1]) && !startsBlock(line) && !ITEM.test(line)) body.push(line.trim());
      else break;
      i++;
    }
    const task = TASK.exec(body[0]);
    if (task) body[0] = body[0].slice(task[0].length);
    items.push({ checked: task ? task[1] !== " " : null, children: blocksFrom(body) });
  }
  return [{ kind: "list", ordered, start: ordered ? parseInt(first[2], 10) : 1, items }, i];
}

function blocksFrom(lines: string[]): Block[] {
  const blocks: Block[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (isBlank(line)) {
      i++;
      continue;
    }
    const fence = FENCE.exec(line);
    if (fence) {
      const code: string[] = [];
      const indent = indentOf(line);
      i++;
      while (i < lines.length && !closesFence(lines[i], fence[1])) {
        code.push(lines[i].slice(Math.min(indent, indentOf(lines[i]))));
        i++;
      }
      i++;
      blocks.push({ kind: "code", lang: fence[2], text: code.join("\n") });
      continue;
    }
    const heading = HEADING.exec(line);
    if (heading) {
      blocks.push({ kind: "heading", level: heading[1].length, children: inlineFrom(heading[2] ?? "") });
      i++;
      continue;
    }
    if (RULE.test(line)) {
      blocks.push({ kind: "rule" });
      i++;
      continue;
    }
    if (QUOTE.test(line)) {
      const inner: string[] = [];
      while (i < lines.length && QUOTE.test(lines[i])) inner.push(lines[i++].replace(QUOTE, ""));
      blocks.push({ kind: "quote", children: blocksFrom(inner) });
      continue;
    }
    if (ITEM.test(line)) {
      const [list, next] = listFrom(lines, i);
      blocks.push(list);
      i = next;
      continue;
    }
    if (line.includes("|") && i + 1 < lines.length && DELIMITER_ROW.test(lines[i + 1]) && lines[i + 1].includes("-")) {
      const head = splitRow(line);
      const delimiter = splitRow(lines[i + 1]);
      if (head.length === delimiter.length) {
        const rows: Inline[][][] = [];
        i += 2;
        while (i < lines.length && !isBlank(lines[i]) && !startsBlock(lines[i])) {
          const cells = splitRow(lines[i++]);
          rows.push(head.map((_, c) => inlineFrom(cells[c] ?? "")));
        }
        blocks.push({ kind: "table", head: head.map(inlineFrom), align: delimiter.map(alignOf), rows });
        continue;
      }
    }
    const paragraph = [line.trim()];
    i++;
    while (i < lines.length && !isBlank(lines[i]) && !startsBlock(lines[i])) paragraph.push(lines[i++].trim());
    blocks.push({ kind: "paragraph", children: inlineFrom(paragraph.join("\n")) });
  }
  return blocks;
}

export const blocksOf = (text: string): Block[] => blocksFrom(text.replace(/\r\n?/g, "\n").replace(/\t/g, "    ").split("\n"));

const isWord = (c: string | undefined) => c !== undefined && /[\p{L}\p{N}]/u.test(c);
const isSpace = (c: string | undefined) => c === undefined || /\s/.test(c);
const isWebLink = (href: string) => /^(https?:\/\/|mailto:)/i.test(href);

// The index of the closing delimiter of an emphasis span that opens at `from`, or -1.
function closerOf(s: string, delimiter: string, from: number): number {
  const mark = delimiter[0];
  for (let j = from; j < s.length; j++) {
    if (s[j] === "\\") j++;
    else if (s[j] === "`") {
      const close = s.indexOf("`", j + 1);
      if (close > 0) j = close;
    } else if (s.startsWith(delimiter, j) && !isSpace(s[j - 1]) && j > from) {
      const after = s[j + delimiter.length];
      if (delimiter.length === 1 && (s[j + 1] === mark || s[j - 1] === mark)) continue;
      if (mark === "_" && isWord(after)) continue;
      let run = delimiter.length;
      while (mark !== "~" && s[j + run] === mark) run++;
      return j + run - delimiter.length;
    }
  }
  return -1;
}

const LINK = /^\[((?:[^[\]\\]|\\.|\[[^\]]*\])*)\]\(\s*<?((?:[^\s()<>]|\([^\s()]*\))*)>?(?:\s+"[^"]*")?\s*\)/;

export function inlineFrom(s: string): Inline[] {
  const out: Inline[] = [];
  let text = "";
  const flush = () => {
    if (text) out.push({ kind: "text", text });
    text = "";
  };
  let i = 0;
  let url: string | null;
  while (i < s.length) {
    const c = s[i];
    if (c === "\\" && s[i + 1] === "\n") {
      flush();
      out.push({ kind: "break" });
      i += 2;
    } else if (c === "\\" && i + 1 < s.length && PUNCTUATION.includes(s[i + 1])) {
      text += s[i + 1];
      i += 2;
    } else if (c === "\n") {
      if (text.endsWith("  ")) {
        text = text.trimEnd();
        flush();
        out.push({ kind: "break" });
      } else text = text.trimEnd() + " ";
      i++;
    } else if (c === "`") {
      let run = 1;
      while (s[i + run] === "`") run++;
      const ticks = "`".repeat(run);
      let close = s.indexOf(ticks, i + run);
      while (close >= 0 && s[close + run] === "`") close = s.indexOf(ticks, close + run + 1);
      if (close < 0) {
        text += ticks;
        i += run;
      } else {
        let code = s.slice(i + run, close).replace(/\n/g, " ");
        if (code.length > 2 && code.startsWith(" ") && code.endsWith(" ")) code = code.slice(1, -1);
        flush();
        out.push({ kind: "code", text: code });
        i = close + run;
      }
    } else if (c === "[") {
      const m = LINK.exec(s.slice(i));
      if (m) {
        const children = inlineFrom(m[1]);
        flush();
        if (isWebLink(m[2])) out.push({ kind: "link", href: m[2], children });
        else out.push(...children);
        i += m[0].length;
      } else {
        text += c;
        i++;
      }
    } else if ((c === "*" || c === "_" || c === "~") && !(c === "_" && isWord(s[i - 1]))) {
      let run = 1;
      while (s[i + run] === c) run++;
      const length = c === "~" ? (run >= 2 ? 2 : 0) : Math.min(run, 3);
      const delimiter = c.repeat(length);
      const close = length > 0 && !isSpace(s[i + length]) ? closerOf(s, delimiter, i + length) : -1;
      if (close < 0) {
        text += c.repeat(run);
        i += run;
      } else {
        const children = inlineFrom(s.slice(i + length, close));
        flush();
        if (c === "~") out.push({ kind: "del", children });
        else if (length === 3) out.push({ kind: "strong", children: [{ kind: "em", children }] });
        else out.push({ kind: length === 2 ? "strong" : "em", children });
        i = close + length;
      }
    } else if (c === "h" && !isWord(s[i - 1]) && (url = webUrlAt(s, i))) {
      flush();
      out.push({ kind: "link", href: url, children: [{ kind: "text", text: url }] });
      i += url.length;
    } else {
      text += c;
      i++;
    }
  }
  flush();
  return out;
}
