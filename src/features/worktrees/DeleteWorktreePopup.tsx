// The sheet that confirms a worktree delete before its folder leaves the disk.
import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { confirmDeletion } from "./store";

export function DeleteWorktreePopup() {
  const pending = sheetOf(sheet.use(), "deleteWorktree");
  if (!pending) return null;
  return (
    <ConfirmSheet
      title="Delete Worktree?"
      description={`Delete ${pending.name}? Its folder is removed from disk.`}
      confirmLabel="Delete"
      onCancel={() => closeSheet("deleteWorktree")}
      onConfirm={confirmDeletion}
    />
  );
}
