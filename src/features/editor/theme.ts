// The editor's look: the app's tokens for the surface, the selection and the syntax colors.
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags } from "@lezer/highlight";

const surface = EditorView.theme({
  "&": { height: "100%", color: "var(--ink)", backgroundColor: "var(--canvas)", fontSize: "12px" },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": { fontFamily: "var(--font-code)", lineHeight: "1.45" },
  ".cm-content": { caretColor: "var(--accent-ink)", padding: "8px 0" },
  ".cm-cursor": { borderLeftColor: "var(--accent-ink)" },
  ".cm-gutters": { backgroundColor: "var(--canvas)", color: "var(--ink-disabled)", border: "none" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--ink-secondary)" },
  ".cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground": { backgroundColor: "var(--editor-selection)" },
  ".cm-matchingBracket": { backgroundColor: "var(--editor-match)", outline: "none" },
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
