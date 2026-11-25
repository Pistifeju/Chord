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
