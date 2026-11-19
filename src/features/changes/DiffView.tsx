// A file's diff in its tab: the read-only unified diff of the index against HEAD, of the file on disk against the index, of the branch since its base, of one commit or of one stash, with the editor's syntax highlighting and each hunk's Stage and Discard, or Unstage and Discard, read again with each read of the Changes view.
import { drawSelection, EditorView, Decoration, gutter, GutterMarker, WidgetType } from "@codemirror/view";
import { Compartment, EditorState, type Range } from "@codemirror/state";
import { LanguageDescription, type LanguageSupport } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { useEffect, useRef, useState } from "react";
import { commands, type DiffLine, type DiffSide, type FileDiff, type Hunk, type HunkAction } from "../../ipc/bindings";
import { fail } from "../../ui/alert";
import { editorTheme } from "../editor/theme";
import { diffChanges, diffKey } from "./changes";
import { applyHunk, changes, requestDiscard } from "./store";

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

type Act = (hunk: number, action: HunkAction) => void;

const actionLabels: Record<HunkAction, string> = { stage: "Stage", unstage: "Unstage", discard: "Discard", discardStaged: "Discard" };

// The band where git left out unchanged lines, naming what the next hunk sits in, with the hunk's buttons.
class GapWidget extends WidgetType {
  constructor(
    readonly context: string,
    readonly hunk: number,
    readonly actions: HunkAction[],
    readonly act: Act,
  ) {
    super();
  }
  eq(other: GapWidget) {
    return other.context === this.context && other.hunk === this.hunk && other.actions.join() === this.actions.join();
  }
  toDOM() {
    const band = document.createElement("div");
    band.className = "cm-diff-gap";
    const context = band.appendChild(document.createElement("span"));
    context.className = "cm-diff-context";
    context.textContent = this.context;
    if (this.actions.length === 0) return band;
    const bar = band.appendChild(document.createElement("span"));
    bar.className = "cm-diff-actions";
    for (const action of this.actions) {
      const button = bar.appendChild(document.createElement("button"));
      button.type = "button";
      button.className = "button-secondary cm-diff-action";
      button.textContent = actionLabels[action];
      button.onclick = () => {
        // A second click would find the hunk gone; the next read rebuilds the band.
        if (action !== "discard") button.disabled = true;
        this.act(this.hunk, action);
      };
    }
    return band;
  }
}

const gapMarker = new GutterText("", "cm-diff-gap");

const diffTheme = EditorView.theme({
  // The sideways scrollbar keeps its track from the start, so a narrower tab never shifts the lines up.
  ".cm-scroller": { overflowX: "scroll" },
  "&.cm-diff-banded .cm-content": { paddingTop: 0 },
  ".cm-diff-added": { backgroundColor: "var(--diff-added)" },
  ".cm-diff-removed": { backgroundColor: "var(--diff-removed)" },
  ".cm-diff-gap": { backgroundColor: "var(--panel)", color: "var(--ink-muted)", lineHeight: "24px" },
  ".cm-content > .cm-diff-gap": { display: "flex", alignItems: "center", height: "24px", paddingLeft: "8px" },
  ".cm-diff-gap > .cm-diff-context": { flex: 1, minWidth: 0, overflow: "hidden", whiteSpace: "pre", textOverflow: "ellipsis" },
  ".cm-diff-gap > .cm-diff-context:last-child": { paddingRight: "8px" },
  // Sticky on the band's fill, so the buttons stay in view and cover the context while a long line scrolls the band sideways.
  ".cm-diff-actions": { position: "sticky", right: 0, display: "flex", flex: "none", gap: "4px", padding: "0 8px", backgroundColor: "var(--panel)" },
  // A compact secondary button: its fill, border, shadow, hover and pressed come from controls.css.
  ".cm-diff-action": {
    height: "20px",
    padding: "0 8px",
    borderRadius: "var(--radius-inset)",
    fontFamily: "var(--font-ui)",
    fontSize: "calc(12px * var(--text-scale))",
    lineHeight: "20px",
    // The band is 24px tall, so the focus ring hugs the button instead of reaching into the code.
    outlineOffset: "0",
  },
  ".cm-diff-action:disabled": { color: "var(--ink-disabled)", boxShadow: "inset 0 0 0 1px var(--line-strong)" },
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

function diffExtensions(diff: FileDiff, actions: HunkAction[], act: Act) {
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
    if (actions.length > 0 || skipsBefore(diff.hunks, h)) marks.push(Decoration.widget({ widget: new GapWidget(hunk.context, h, actions, act), block: true, side: -1 }).range(from));
    hunk.lines.forEach((line) => {
      if (line.kind !== "context") marks.push(Decoration.line({ class: `cm-diff-${line.kind}` }).range(from));
      from += line.text.length + 1;
    });
  });
  // A band on the first line takes the place of the content's top padding.
  const banded = marks[0]?.from === 0 && marks[0].value.spec.block ? EditorView.editorAttributes.of({ class: "cm-diff-banded" }) : [];
  return { doc: lines.map((line) => line.text).join("\n"), extensions: [numbers("old"), numbers("new"), sign, EditorView.decorations.of(Decoration.set(marks, true)), banded] };
}

const sameDiff = (a: FileDiff | null, b: FileDiff) => JSON.stringify(a) === JSON.stringify(b);

export function DiffView({ worktree, path, side }: { worktree: string; path: string; side: DiffSide }) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView>(undefined);
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [failed, setFailed] = useState(false);
  const read = changes.usePart((all) => all[worktree]);
  // A branch's, a commit's or a stash's diff is committed, so it has no hunk to stage or discard.
  const change = typeof side === "string" ? read?.find((c) => c.path === path && c.staged === (side === "staged")) : undefined;
  // A conflicted file stages whole, which marks it resolved.
  const actions: HunkAction[] = !change || change.status === "conflicted" ? [] : side === "staged" ? ["discardStaged", "unstage"] : ["discard", "stage"];
  const actionKey = actions.join();
  const latest = useRef({ worktree, path, diff, change });
  latest.current = { worktree, path, diff, change };
  const act = useRef<Act>((h, action) => {
    const { worktree, path, diff, change } = latest.current;
    const hunk = diff?.hunks[h];
    if (!hunk || !change) return;
    if (action === "discard" || action === "discardStaged") requestDiscard(worktree, [change], hunk);
    else void applyHunk(worktree, path, hunk, action);
  });
  const shown = useRef(false);
  // A new tab state brings a new object for the same side, so the read follows its key.
  const key = diffKey(side, path);
  const refresh = diffChanges(side) ? read : null;
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
  }, [worktree, key, refresh]);
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
    const { doc, extensions } = diffExtensions(diff, actionKey ? (actionKey.split(",") as HunkAction[]) : [], act.current);
    const top = view.scrollDOM.scrollTop;
    view.setState(
      EditorState.create({
        doc,
        extensions: [EditorState.lineSeparator.of("\n"), EditorState.readOnly.of(true), drawSelection(), language.current.of(support.current ?? []), editorTheme, diffTheme, extensions],
      }),
    );
    view.scrollDOM.scrollTop = top;
  }, [diff, actionKey]);
  const note = diff ? (diff.binary ? "Binary file" : diff.hunks.length === 0 ? "No changes" : null) : failed ? "No diff to show" : null;
  return (
    <div className="editor" data-selectable>
      <div ref={host} className="editor-host" />
      {note && <p className="diff-note">{note}</p>}
      {!diff && !failed && <span className="spinner" role="status" aria-label="Loading" />}
    </div>
  );
}
