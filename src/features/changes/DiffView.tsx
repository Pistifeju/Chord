// A file's diff in its tab: the read-only unified diff of the index against HEAD or of the file on disk against the index, with the editor's syntax highlighting, read again with each read of the Changes view.
import { drawSelection, EditorView, Decoration, gutter, GutterMarker, WidgetType } from "@codemirror/view";
import { Compartment, EditorState, type Range } from "@codemirror/state";
import { LanguageDescription, type LanguageSupport } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { useEffect, useRef, useState } from "react";
import { commands, type DiffLine, type DiffSide, type FileDiff, type Hunk } from "../../ipc/bindings";
import { fail } from "../../ui/alert";
import { editorTheme } from "../editor/theme";
import { changes } from "./store";

class GutterText extends GutterMarker {
  constructor(
    readonly text: string,
    readonly elementClass = "",
  ) {
    super();
  }
  eq(other: GutterText) {
    return other.text === this.text && other.elementClass === this.elementClass;
  }
  toDOM() {
    return document.createTextNode(this.text);
  }
}

// The band where git left out unchanged lines, naming what the next hunk sits in.
class GapWidget extends WidgetType {
  constructor(readonly context: string) {
    super();
  }
  eq(other: GapWidget) {
    return other.context === this.context;
  }
  toDOM() {
    const band = document.createElement("div");
    band.className = "cm-diff-gap";
    band.textContent = this.context;
    return band;
  }
}

const gapMarker = new GutterText("", "cm-diff-gap");

const diffTheme = EditorView.theme({
  ".cm-diff-added": { backgroundColor: "var(--diff-added)" },
  ".cm-diff-removed": { backgroundColor: "var(--diff-removed)" },
  ".cm-diff-gap": { backgroundColor: "var(--panel)", color: "var(--ink-muted)", lineHeight: "24px" },
  ".cm-content > .cm-diff-gap": { height: "24px", padding: "0 8px", overflow: "hidden", whiteSpace: "pre", textOverflow: "ellipsis" },
  ".cm-diff-number .cm-gutterElement": { padding: "0 6px 0 4px", textAlign: "right" },
  ".cm-diff-number .cm-diff-added, .cm-diff-number .cm-diff-removed": { color: "var(--ink-muted)" },
  ".cm-diff-sign .cm-gutterElement": { width: "12px", textAlign: "center" },
  ".cm-diff-sign .cm-diff-added": { color: "var(--success)" },
  ".cm-diff-sign .cm-diff-removed": { color: "var(--danger)" },
});

const signs: Record<DiffLine["kind"], string> = { context: "", added: "+", removed: "−" };

// Whether git left out lines before a hunk, judged on whichever side both hunks number.
function skipsBefore(hunks: Hunk[], h: number): boolean {
  for (const side of ["old", "new"] as const) {
    const first = hunks[h].lines.find((line) => line[side] != null)?.[side];
    const last = h === 0 ? 0 : [...hunks[h - 1].lines].reverse().find((line) => line[side] != null)?.[side];
    if (first != null && last != null) return first - last > 1;
  }
  return h > 0;
}

function diffExtensions(diff: FileDiff) {
  const lines = diff.hunks.flatMap((hunk) => hunk.lines);
  const lineAt = (view: EditorView, from: number) => lines[view.state.doc.lineAt(from).number - 1];
  const widest = String(Math.max(0, ...lines.map((line) => Math.max(line.old ?? 0, line.new ?? 0))));
  // Each gutter cell takes its line's wash, so a changed line reads across its numbers.
  const cells = (cls: string, text: (line: DiffLine | undefined) => string, spacer?: string) =>
    gutter({
      class: cls,
      lineMarker: (view, block) => {
        const line = lineAt(view, block.from);
        return new GutterText(text(line), `cm-diff-${line?.kind ?? "context"}`);
      },
      widgetMarker: () => gapMarker,
      initialSpacer: spacer === undefined ? undefined : () => new GutterText(spacer),
    });
  const numbers = (side: "old" | "new") => cells("cm-diff-number", (line) => String(line?.[side] ?? ""), widest);
  const sign = cells("cm-diff-sign", (line) => signs[line?.kind ?? "context"]);
  const marks: Range<Decoration>[] = [];
  let from = 0;
  diff.hunks.forEach((hunk, h) => {
    if (skipsBefore(diff.hunks, h)) marks.push(Decoration.widget({ widget: new GapWidget(hunk.context), block: true, side: -1 }).range(from));
    hunk.lines.forEach((line) => {
      if (line.kind !== "context") marks.push(Decoration.line({ class: `cm-diff-${line.kind}` }).range(from));
      from += line.text.length + 1;
    });
  });
  return { doc: lines.map((line) => line.text).join("\n"), extensions: [numbers("old"), numbers("new"), sign, EditorView.decorations.of(Decoration.set(marks, true))] };
}

const sameDiff = (a: FileDiff | null, b: FileDiff) => JSON.stringify(a) === JSON.stringify(b);

export function DiffView({ worktree, path, side }: { worktree: string; path: string; side: DiffSide }) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView>(undefined);
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [failed, setFailed] = useState(false);
  const read = changes.usePart((all) => all[worktree]);
  const shown = useRef(false);
  useEffect(() => {
    let gone = false;
    commands
      .readDiff(worktree, path, side)
      .then((next) => {
        if (gone) return;
        shown.current = true;
        setDiff((current) => (sameDiff(current, next) ? current : next));
      })
      .catch((error) => {
        // Only the first read reports; a refresh that fails keeps the last diff quietly.
        if (gone || shown.current) return;
        shown.current = true;
        fail(error);
        setFailed(true);
      });
    return () => void (gone = true);
  }, [worktree, path, side, read]);
  // One view for the tab; a new diff replaces its state and keeps the scroll.
  const language = useRef(new Compartment());
  const support = useRef<LanguageSupport>(undefined);
  useEffect(() => {
    if (!host.current) return;
    const view = new EditorView({ parent: host.current });
    viewRef.current = view;
    void LanguageDescription.matchFilename(languages, path)
      ?.load()
      .then((loaded) => {
        support.current = loaded;
        if (viewRef.current === view) view.dispatch({ effects: language.current.reconfigure(loaded) });
      });
    return () => {
      view.destroy();
      viewRef.current = undefined;
    };
  }, [path]);
  useEffect(() => {
    const view = viewRef.current;
    if (!view || !diff) return;
    const { doc, extensions } = diffExtensions(diff);
    const top = view.scrollDOM.scrollTop;
    view.setState(
      EditorState.create({
        doc,
        extensions: [EditorState.lineSeparator.of("\n"), EditorState.readOnly.of(true), drawSelection(), language.current.of(support.current ?? []), editorTheme, diffTheme, extensions],
      }),
    );
    view.scrollDOM.scrollTop = top;
  }, [diff]);
  const note = diff ? (diff.binary ? "Binary file" : diff.hunks.length === 0 ? "No changes" : null) : failed ? "No diff to show" : null;
  return (
    <div className="editor" data-selectable>
      <div ref={host} className="editor-host" />
      {note && <p className="diff-note">{note}</p>}
      {!diff && !failed && <span className="spinner" role="status" aria-label="Loading" />}
    </div>
  );
}
