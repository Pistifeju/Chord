// Drop target: folders dropped on the window become projects.
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useState } from "react";
import { addFolders } from "./store";
import "./worktrees.css";

export function ProjectDrop() {
  const [over, setOver] = useState(false);
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      setOver(payload.type === "enter" || payload.type === "over");
      if (payload.type === "drop") addFolders(payload.paths);
    });
    return () => void unlisten.then((stop) => stop());
  }, []);
  if (!over) return null;
  return <div className="project-drop">Drop a folder to add it as a project</div>;
}
