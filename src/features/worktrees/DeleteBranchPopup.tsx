// The sheets that ask before a local branch is deleted, and again with force when git calls it unmerged.
import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { confirmDeleteBranch } from "./localBranches";

export function DeleteBranchPopup() {
  const pending = sheetOf(sheet.use(), "deleteBranch");
  if (!pending) return null;
  return (
    <ConfirmSheet
      title={pending.unmerged ? "Delete Unmerged Branch?" : "Delete Branch?"}
      description={pending.unmerged ? `${pending.branch} has commits that are not merged. Deleting it loses them.` : `Chord deletes the branch ${pending.branch}.`}
      confirmLabel="Delete"
      onCancel={() => closeSheet("deleteBranch")}
      onConfirm={() => void confirmDeleteBranch()}
    />
  );
}
