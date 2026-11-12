// Empty state while Chord has no project.
import { tooltip } from "../../app/commands";
import { shortcutChanges } from "../settings/store";
import { FolderIcon } from "../../ui/icons";
import { pickProject } from "./store";
import "./worktrees.css";

export function NoProjects() {
  shortcutChanges.use();
  return (
    <div className="no-projects">
      <span className="no-projects-icon">
        <FolderIcon />
      </span>
      <h1 className="no-projects-title">No projects yet</h1>
      <p className="no-projects-text">Add a git repository, or drop its folder on the window.</p>
      <button type="button" className="button-primary" title={tooltip("project.add")} onClick={pickProject}>
        Add Project
      </button>
    </div>
  );
}
