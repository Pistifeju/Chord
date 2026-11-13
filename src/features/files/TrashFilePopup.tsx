// The sheet that asks before a file or folder of the Files tree moves to the Trash.
import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { confirmTrash } from "./entries";

export function TrashFilePopup() {
  const pending = sheetOf(sheet.use(), "trashFile");
  if (!pending) return null;
  return (
    <ConfirmSheet
      title="Move to Trash?"
      description={`Chord moves ${pending.path.split("/").pop()} to the Trash.`}
      detail={pending.path}
      confirmLabel="Move to Trash"
      onCancel={() => closeSheet("trashFile")}
      onConfirm={() => void confirmTrash()}
    />
  );
}
