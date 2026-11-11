// The sheet that confirms a worktree delete before its folder leaves the disk.
import { cancelDeletion, confirmDeletion, deletion } from "./store";

export function DeleteWorktreePopup() {
  const pending = deletion.use();
  if (!pending) return null;
  return (
    <div className="sheet-scrim" onMouseDown={cancelDeletion}>
      <div className="sheet" role="alertdialog" aria-label="Delete Worktree" onMouseDown={(e) => e.stopPropagation()}>
        <h2 className="sheet-title">Delete Worktree?</h2>
        <p className="sheet-description">Delete {pending.name}? Its folder is removed from disk.</p>
        <footer className="sheet-actions">
          <button type="button" className="pill-secondary" onClick={cancelDeletion}>
            Cancel
          </button>
          <button type="button" className="pill-destructive" autoFocus onClick={confirmDeletion}>
            Delete
          </button>
        </footer>
      </div>
    </div>
  );
}
