// The editor of the open file: CodeMirror with syntax highlighting, saved as you type.
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput, LanguageDescription } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { Compartment, EditorState } from "@codemirror/state";
import { drawSelection, EditorView, keymap, lineNumbers } from "@codemirror/view";
import { useEffect, useRef, useState } from "react";
import { fail } from "../worktrees/store";
import { closeFile, flushSave, readFile, scheduleSave } from "./store";
import { editorTheme } from "./theme";
import "./editor.css";

export function Editor({ worktree, path }: { worktree: string; path: string }) {
  const host = useRef<HTMLDivElement>(null);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let view: EditorView | undefined;
    let gone = false;
    readFile(worktree, path)
      .then((text) => {
        if (gone || !host.current) return;
        setLoading(false);
        const language = new Compartment();
        view = new EditorView({
          parent: host.current,
          state: EditorState.create({
            doc: text,
            extensions: [
              lineNumbers(),
              history(),
              drawSelection(),
              indentOnInput(),
              bracketMatching(),
              keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
              language.of([]),
              editorTheme,
              EditorView.updateListener.of((update) => {
                if (update.docChanged) scheduleSave(worktree, path, update.state.doc);
              }),
            ],
          }),
        });
        view.focus();
        const description = LanguageDescription.matchFilename(languages, path);
        void description?.load().then((support) => view?.dispatch({ effects: language.reconfigure(support) }));
      })
      .catch((error) => {
        if (gone) return;
        fail(error);
        closeFile(worktree, path);
      });
    return () => {
      gone = true;
      view?.destroy();
      flushSave(worktree, path);
    };
  }, [worktree, path]);
  return (
    <div className="editor" data-selectable>
      <div ref={host} className="editor-host" />
      {loading && <span className="spinner" role="status" aria-label="Loading" />}
    </div>
  );
}
