// The view of an open image file: the image fit to the work area, read again when the file changes on disk.
import { useEffect, useState } from "react";
import { commands } from "../../ipc/bindings";
import { imageUrl } from "../chat/attachments";
import { fail } from "../worktrees/store";
import { closeFile } from "./store";
import "./editor.css";

/** Milliseconds between two looks at the file's change time. */
const CHANGE_CHECK_DELAY = 1000;

export function ImageView({ worktree, path, mime }: { worktree: string; path: string; mime: string }) {
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
        // A file gone from disk closes its tab; only the first read reports why.
        if (shown === null) fail(error);
        closeFile(worktree, path);
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
