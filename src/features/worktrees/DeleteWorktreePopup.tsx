// The sheet that confirms a worktree delete before its folder leaves the disk.
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { cancelDeletion, confirmDeletion, deletion } from "./store";

export function DeleteWorktreePopup() {
  const pending = deletion.use();
  if (!pending) return null;
  return (
    <ConfirmSheet
      title="Delete Worktree?"
      description={`Delete ${pending.name}? Its folder is removed from disk.`}
      confirmLabel="Delete"
      onCancel={cancelDeletion}
      onConfirm={confirmDeletion}
    />
  );
}
