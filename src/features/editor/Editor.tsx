// The editor of the open file: CodeMirror with syntax highlighting, saved as you type, read again when an agent changes the file.
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput, LanguageDescription, type LanguageSupport } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { Compartment, EditorState } from "@codemirror/state";
import { drawSelection, EditorView, keymap, lineNumbers } from "@codemirror/view";
import { useEffect, useRef, useState } from "react";
import { commands } from "../../ipc/bindings";
import { fail } from "../worktrees/store";
import { changedOnDisk, flushSave, readFile, scheduleSave } from "./store";
import { editorTheme } from "./theme";

/** Milliseconds between two looks at the file's change time. */
const CHANGE_CHECK_DELAY = 1000;

// `onGone` closes whatever shows the file, a tab or a pane, once it cannot be read; `autoFocus` is off for a pane the focus is not on, so it does not take it from the agent.
export function Editor({ worktree, path, autoFocus, onGone }: { worktree: string; path: string; autoFocus: boolean; onGone: () => void }) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView>(undefined);
  // The load finishes later than the render, so it reads the focus the pane has then.
  const focusRef = useRef(autoFocus);
  focusRef.current = autoFocus;
  const [loading, setLoading] = useState(true);
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
          EditorView.updateListener.of((update) => {
            if (update.docChanged) scheduleSave(worktree, path, update.state.doc, update.state.lineBreak);
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
      flushSave(worktree, path);
    };
  }, [worktree, path]);
  useEffect(() => {
    if (autoFocus) viewRef.current?.focus();
  }, [autoFocus]);
  return (
    <div className="editor" data-selectable>
      <div ref={host} className="editor-host" />
      {loading && <span className="spinner" role="status" aria-label="Loading" />}
    </div>
  );
}
