import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { confirmTrash } from "./entries";
import { fileName } from "./paths";

export function TrashFilePopup() {
  const pending = sheetOf(sheet.use(), "trashFile");
  if (!pending) return null;
  return (
    <ConfirmSheet
      title="Move to Trash?"
      description={`Chord moves ${fileName(pending.path)} to the Trash.`}
      detail={pending.path}
      confirmLabel="Move to Trash"
      onCancel={() => closeSheet("trashFile")}
      onConfirm={() => void confirmTrash()}
    />
  );
}
