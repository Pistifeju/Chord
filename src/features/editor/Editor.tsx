import { autocompletion, closeBrackets, closeBracketsKeymap, completeAnyWord, completionKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, foldGutter, foldKeymap, indentOnInput, LanguageDescription, type LanguageSupport } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { highlightSelectionMatches } from "@codemirror/search";
import { Compartment, EditorState } from "@codemirror/state";
import { crosshairCursor, drawSelection, EditorView, highlightActiveLine, keymap, lineNumbers, rectangularSelection } from "@codemirror/view";
import { useEffect, useId, useRef, useState } from "react";
import { commands } from "../../ipc/bindings";
import { fail } from "../../ui/alert";
import { EditorFind } from "./EditorFind";
import { closeEditorFind, editorFinds } from "./find";
import { findMarks } from "./findMarks";
import { keptFolds } from "./folds";
import { requestedLine, lineRequests, takeLine } from "./reveal";
import { changedOnDisk, closeFile, readFile, scheduleSave } from "./store";
import { editorTheme } from "./theme";

const CHANGE_CHECK_DELAY = 1000;

export function Editor({ worktree, path, autoFocus, onGone, onEdit }: { worktree: string; path: string; autoFocus: boolean; onGone: () => void; onEdit: () => void }) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView>(undefined);
  const focusRef = useRef(autoFocus);
  focusRef.current = autoFocus;
  const [loading, setLoading] = useState(true);
  const id = useId();
  const find = editorFinds.usePart((all) => all[id]);
  const finding = useRef(false);
  finding.current = find !== undefined;
  const [version, setVersion] = useState(0);
  useEffect(() => {
    let view: EditorView | undefined;
    let gone = false;
    const language = new Compartment();
    let support: LanguageSupport | undefined;
    const stateOf = (text: string) => {
      const lineBreak = text.includes("\r\n") ? [EditorState.lineSeparator.of("\r\n")] : [];
      return EditorState.create({
        doc: text,
        extensions: [
          lineBreak,
          lineNumbers(),
          foldGutter({ openText: "⌄", closedText: "›" }),
          history(),
          drawSelection(),
          EditorState.allowMultipleSelections.of(true),
          rectangularSelection(),
          crosshairCursor(),
          highlightActiveLine(),
          highlightSelectionMatches(),
          indentOnInput(),
          bracketMatching(),
          closeBrackets(),
          autocompletion({ icons: false }),
          EditorState.languageData.of(() => [{ autocomplete: completeAnyWord }]),
          keymap.of([...closeBracketsKeymap, ...completionKeymap, ...defaultKeymap, ...historyKeymap, ...foldKeymap, indentWithTab]),
          language.of(support ?? []),
          editorTheme,
          findMarks,
          EditorView.updateListener.of((update) => {
            if (!update.docChanged) return;
            if (finding.current) setVersion((v) => v + 1);
            scheduleSave(worktree, path, update.state.doc, update.state.lineBreak);
            onEdit();
          }),
        ],
      });
    };
    readFile(worktree, path)
      .then((text) => {
        if (gone || !host.current) return;
        setLoading(false);
        view = new EditorView({ parent: host.current, state: stateOf(text) });
        viewRef.current = view;
        if (focusRef.current) view.focus();
        const description = LanguageDescription.matchFilename(languages, path);
        void description?.load().then((loaded) => {
          support = loaded;
          view?.dispatch({ effects: language.reconfigure(loaded) });
        });
      })
      .catch((error) => {
        if (gone) return;
        fail(error);
        onGone();
      });
    let seen: string | null = null;
    let checking = false;
    const timer = window.setInterval(async () => {
      if (!view || checking || document.visibilityState !== "visible") return;
      checking = true;
      try {
        const modified = await commands.fileModified(worktree, path);
        if (modified === seen) return;
        seen = modified;
        const text = await changedOnDisk(worktree, path);
        if (text === null || gone || !view) return;
        const { anchor, head } = view.state.selection.main;
        const state = stateOf(text);
        const clamp = (at: number) => Math.min(at, state.doc.length);
        const folds = keptFolds(view.state, state);
        view.setState(state);
        view.dispatch({ selection: { anchor: clamp(anchor), head: clamp(head) }, effects: folds });
        setVersion((v) => v + 1);
      } catch {
      } finally {
        checking = false;
      }
    }, CHANGE_CHECK_DELAY);
    return () => {
      gone = true;
      window.clearInterval(timer);
      view?.destroy();
      viewRef.current = undefined;
      closeEditorFind(id);
      closeFile(worktree, path);
    };
  }, [worktree, path, id]);
  useEffect(() => {
    if (autoFocus) viewRef.current?.focus();
  }, [autoFocus]);
  const line = lineRequests.usePart((all) => requestedLine(all, worktree, path));
  useEffect(() => {
    const view = viewRef.current;
    if (line === undefined || !view) return;
    const from = view.state.doc.line(Math.min(Math.max(line, 1), view.state.doc.lines)).from;
    view.dispatch({ selection: { anchor: from }, effects: EditorView.scrollIntoView(from, { y: "center" }) });
    takeLine(worktree, path);
  }, [line, loading, worktree, path]);
  return (
    <div className="editor" data-selectable data-find={id}>
      <div ref={host} className="editor-host" />
      {find && viewRef.current && <EditorFind id={id} view={viewRef.current} find={find} version={version} />}
      {loading && <span className="spinner" role="status" aria-label="Loading" />}
    </div>
  );
}
