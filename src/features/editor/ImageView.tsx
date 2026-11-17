// The view of an open image file: the image fit to the work area, read again when the file changes on disk.
import { useEffect, useState } from "react";
import { commands } from "../../ipc/bindings";
import { imageUrl } from "../chat/attachments";
import { fail } from "../../ui/alert";
import "./editor.css";

/** Milliseconds between two looks at the file's change time. */
const CHANGE_CHECK_DELAY = 1000;

// `onGone` closes whatever shows the file, a tab or a pane, once it is gone from disk.
export function ImageView({ worktree, path, mime, onGone }: { worktree: string; path: string; mime: string; onGone: () => void }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    let gone = false;
    let shown: string | null = null;
    let checking = false;
    async function check() {
      if (checking) return;
      checking = true;
      try {
        const modified = await commands.fileModified(worktree, path);
        if (gone || modified === shown) return;
        const data = await commands.readImageFile(`${worktree}/${path}`);
        if (gone) return;
        shown = modified;
        setUrl(imageUrl({ data, mime_type: mime }));
      } catch (error) {
        if (gone) return;
        // A file gone from disk closes its tab or pane; only the first read reports why.
        if (shown === null) fail(error);
        onGone();
      } finally {
        checking = false;
      }
    }
    void check();
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") void check();
    }, CHANGE_CHECK_DELAY);
    return () => {
      gone = true;
      window.clearInterval(timer);
    };
  }, [worktree, path, mime]);
  return (
    <div className="editor image-view">
      {url ? <img src={url} alt={path} /> : <span className="spinner" role="status" aria-label="Loading" />}
    </div>
  );
}
