// The editor's look: the app's tokens for the surface, the selection, the fold markers, the completion list and the syntax colors.
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";

const surface = EditorView.theme({
  "&": { height: "100%", color: "var(--ink)", backgroundColor: "var(--canvas)", fontSize: "var(--editor-font-size)" },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": { fontFamily: "var(--font-code)", lineHeight: "1.45" },
  ".cm-content": { caretColor: "var(--accent-ink)", padding: "8px 0" },
  ".cm-cursor": { borderLeftColor: "var(--accent-ink)" },
  // Its own layer: WebKit otherwise paints the sticky gutter late on a sideways scroll and the text shows through.
  // The shadow covers the strip at its left that WebKit leaves open while the sticky gutter catches up.
  ".cm-gutters": { backgroundColor: "var(--canvas)", color: "var(--ink-disabled)", border: "none", transform: "translateZ(0)", boxShadow: "-16px 0 0 var(--canvas)" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--ink-secondary)" },
  ".cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground": { backgroundColor: "var(--editor-selection)" },
  ".cm-activeLine": { backgroundColor: "var(--editor-line)" },
  ".cm-matchingBracket": { backgroundColor: "var(--editor-match)", outline: "none" },
  ".cm-selectionMatch": { backgroundColor: "var(--editor-match)" },
  ".cm-foldGutter .cm-gutterElement": { width: "12px", color: "var(--ink-muted)", cursor: "pointer", textAlign: "center" },
  ".cm-foldPlaceholder": { margin: "0 2px", padding: "0 4px", border: "none", borderRadius: "var(--radius-inset)", backgroundColor: "var(--raised)", color: "var(--ink-muted)" },
  ".cm-tooltip.cm-tooltip-autocomplete": { padding: "4px", border: "1px solid var(--line)", borderRadius: "var(--radius-control)", backgroundColor: "var(--sheet)", boxShadow: "var(--shadow-popover)" },
  ".cm-tooltip.cm-tooltip-autocomplete > ul": { maxHeight: "240px", fontFamily: "var(--font-code)" },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li": { padding: "0 8px", lineHeight: "24px", borderRadius: "var(--radius-inset)", color: "var(--ink-secondary)" },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li[aria-selected]": { backgroundColor: "var(--selected)", color: "var(--ink)" },
  ".cm-completionMatchedText": { textDecoration: "none", fontWeight: "600" },
  ".cm-completionDetail": { color: "var(--ink-muted)", fontStyle: "normal" },
  ".cm-find-match": { backgroundColor: "var(--editor-match)" },
  ".cm-find-current": { textDecoration: "underline 2px var(--accent-ink)" },
});

const colors = HighlightStyle.define([
  { tag: [tags.keyword, tags.modifier, tags.operatorKeyword, tags.controlKeyword], color: "var(--syn-keyword)" },
  { tag: [tags.string, tags.special(tags.string), tags.regexp], color: "var(--syn-string)" },
  { tag: [tags.number, tags.bool, tags.null, tags.atom], color: "var(--syn-number)" },
  { tag: [tags.typeName, tags.className, tags.namespace], color: "var(--syn-type)" },
  { tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.definition(tags.variableName)], color: "var(--syn-function)" },
  { tag: [tags.comment, tags.meta], color: "var(--ink-muted)", fontStyle: "italic" },
  { tag: tags.heading, color: "var(--ink)", fontWeight: "600" },
  { tag: tags.link, textDecoration: "underline" },
]);

export const editorTheme = [surface, syntaxHighlighting(colors)];
