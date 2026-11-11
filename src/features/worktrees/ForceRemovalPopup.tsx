// The sheet that asks before a worktree git refused to remove is removed with force.
import { useEscape } from "../../ui/escapeKey";
import { cancelForceRemoval, confirmForceRemoval, forceRemoval } from "./store";

export function ForceRemovalPopup() {
  const pending = forceRemoval.use();
  useEscape(pending !== null, cancelForceRemoval);
  if (!pending) return null;
  return (
    <div className="sheet-scrim" onMouseDown={cancelForceRemoval}>
      <div className="sheet" role="alertdialog" aria-label="Force Delete" onMouseDown={(e) => e.stopPropagation()}>
        <h2 className="sheet-title">Force Delete?</h2>
        <p className="sheet-description">Deleting {pending.branch ?? pending.path} anyway discards its uncommitted changes.</p>
        <p className="sheet-meta">{pending.reason}</p>
        <footer className="sheet-actions">
          <button type="button" className="pill-secondary" onClick={cancelForceRemoval}>
            Cancel
          </button>
          <button type="button" className="pill-destructive" autoFocus onClick={confirmForceRemoval}>
            Delete
          </button>
        </footer>
      </div>
    </div>
  );
}
