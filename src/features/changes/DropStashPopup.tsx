import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { confirmDropStash } from "./store";

export function DropStashPopup() {
  const pending = sheetOf(sheet.use(), "dropStash");
  if (!pending) return null;
  const { message, branch } = pending.stash;
  return (
    <ConfirmSheet
      title="Drop Stash?"
      description={`Chord deletes this stash${branch ? ` made on ${branch}` : ""}. Its changes are lost.`}
      detail={message}
      confirmLabel="Drop"
      onCancel={() => closeSheet("dropStash")}
      onConfirm={() => void confirmDropStash()}
    />
  );
}
