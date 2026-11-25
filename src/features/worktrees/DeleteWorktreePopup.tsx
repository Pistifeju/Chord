import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { confirmDeletion } from "./store";

export function DeleteWorktreePopup() {
  const pending = sheetOf(sheet.use(), "deleteWorktree");
  if (!pending) return null;
  return (
    <ConfirmSheet
      title="Delete Worktree?"
      description={`Chord removes the folder of ${pending.name} from disk.`}
      confirmLabel="Delete"
      onCancel={() => closeSheet("deleteWorktree")}
      onConfirm={confirmDeletion}
    />
  );
}
