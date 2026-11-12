// Drop target: folders dropped on the window become projects.
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useState } from "react";
import { imageMime } from "../chat/attachments";
import { addFolders } from "./store";

export function ProjectDrop() {
  const [over, setOver] = useState(false);
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      // Images belong to the chat they land on, so they neither show the label nor become projects.
      if (payload.type === "enter") setOver(payload.paths.some((path) => !imageMime(path)));
      else if (payload.type !== "over") setOver(false);
      if (payload.type === "drop") addFolders(payload.paths.filter((path) => !imageMime(path)));
    });
    return () => void unlisten.then((stop) => stop());
  }, []);
  if (!over) return null;
  return (
    <div className="drop-target">
      <span className="drop-label">Drop a folder to add it as a project</span>
    </div>
  );
}
