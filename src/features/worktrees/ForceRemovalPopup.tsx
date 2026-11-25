import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { confirmForceRemoval } from "./store";

export function ForceRemovalPopup() {
  const pending = sheetOf(sheet.use(), "forceRemoval");
  if (!pending) return null;
  return (
    <ConfirmSheet
      title="Force Delete?"
      description={`Deleting ${pending.branch ?? pending.path} anyway discards its uncommitted changes.`}
      detail={pending.reason}
      confirmLabel="Delete"
      onCancel={() => closeSheet("forceRemoval")}
      onConfirm={confirmForceRemoval}
    />
  );
}
