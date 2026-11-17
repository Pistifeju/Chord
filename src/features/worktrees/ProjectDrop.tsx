// Drop target: folders dropped on the window become projects.
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useState } from "react";
import { imageMime } from "../chat/attachments";
import { isOverTerminal } from "../terminal/drop";
import { elementAtDrop } from "../../ui/dropPoint";
import { addFolders } from "./store";

export function ProjectDrop() {
  const [over, setOver] = useState(false);
  useEffect(() => {
    let carriesFolders = false;
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      // Images belong to the chat they land on, and every file dropped on a chat or the terminal belongs to it, so they neither show the label nor become projects.
      const claimed = payload.type !== "leave" && (isOverTerminal(payload.position) || !!elementAtDrop(payload.position)?.closest(".chat"));
      if (payload.type === "enter") carriesFolders = payload.paths.some((path) => !imageMime(path));
      setOver(payload.type !== "drop" && payload.type !== "leave" && carriesFolders && !claimed);
      if (payload.type === "drop" && !claimed) addFolders(payload.paths.filter((path) => !imageMime(path)));
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
