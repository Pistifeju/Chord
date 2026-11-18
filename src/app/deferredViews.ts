// The editor, the diff view and the terminal view, loaded apart from the main bundle so the window parses CodeMirror and xterm while launch waits, not before it starts.
import { externalStore } from "../ui/externalStore";

function deferred<M>(load: () => Promise<M>) {
  const loaded = externalStore<M | null>(null);
  let loading: Promise<void> | null = null;
  const start = () =>
    (loading ??= load().then(loaded.set, (error: unknown) => {
      loading = null;
      throw error;
    }));
  return {
    start,
    // Null until loaded; a view that renders before launch loaded it starts the load.
    use: () => {
      const module = loaded.use();
      if (!module) start().catch(console.error);
      return module;
    },
  };
}

export const editorView = deferred(() => import("../features/editor/Editor"));
export const diffView = deferred(() => import("../features/changes/DiffView"));
export const terminalView = deferred(() => import("../features/terminal/TerminalView"));

export const loadDeferredViews = () => Promise.all([editorView.start(), terminalView.start()]).catch(console.error);
