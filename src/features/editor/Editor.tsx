// The editor of the open file: CodeMirror with syntax highlighting, saved as you type, read again when an agent changes the file.
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput, LanguageDescription, type LanguageSupport } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { Compartment, EditorState } from "@codemirror/state";
import { drawSelection, EditorView, keymap, lineNumbers } from "@codemirror/view";
import { useEffect, useId, useRef, useState } from "react";
import { commands } from "../../ipc/bindings";
import { fail } from "../../ui/alert";
import { EditorFind } from "./EditorFind";
import { closeEditorFind, editorFinds } from "./find";
import { findMarks } from "./findMarks";
import { requestedLine, lineRequests, takeLine } from "./reveal";
import { changedOnDisk, closeFile, readFile, scheduleSave } from "./store";
import { editorTheme } from "./theme";

/** Milliseconds between two looks at the file's change time. */
const CHANGE_CHECK_DELAY = 1000;

// `onGone` closes whatever shows the file, a tab or a pane, once it cannot be read; `onEdit` keeps a preview tab once you type; `autoFocus` is off for a pane the focus is not on, so it does not take it from the agent.
export function Editor({ worktree, path, autoFocus, onGone, onEdit }: { worktree: string; path: string; autoFocus: boolean; onGone: () => void; onEdit: () => void }) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView>(undefined);
  // The load finishes later than the render, so it reads the focus the pane has then.
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
      // A CRLF file keeps its line breaks; CodeMirror would write LF otherwise.
      const lineBreak = text.includes("\r\n") ? [EditorState.lineSeparator.of("\r\n")] : [];
      return EditorState.create({
        doc: text,
        extensions: [
          lineBreak,
          lineNumbers(),
          history(),
          drawSelection(),
          indentOnInput(),
          bracketMatching(),
          keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
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
    // An agent's change on disk replaces the text while the editor holds no unsaved edit.
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
        view.setState(state);
        view.dispatch({ selection: { anchor: clamp(anchor), head: clamp(head) } });
        setVersion((v) => v + 1);
      } catch {
        // A file gone from disk keeps its text in the editor until the tab closes.
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
  // A click on a build error asks for a line; it waits here until the file is read.
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
