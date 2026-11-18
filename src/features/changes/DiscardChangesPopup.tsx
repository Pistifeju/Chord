// The sheet that asks before changes on disk or in the index, or one hunk of them, are discarded.
import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { fileName } from "../files/paths";
import { confirmDiscard, goesToTrash } from "./store";

const files = (count: number) => (count === 1 ? "1 file" : `${count} files`);

export function DiscardChangesPopup() {
  const pending = sheetOf(sheet.use(), "discardChanges");
  if (!pending) return null;
  const { changes, hunk } = pending;
  const untracked = changes.filter(goesToTrash).length;
  const restored = changes.length - untracked;
  const one = changes.length === 1 ? changes[0] : null;
  const source = changes[0].staged ? "the last commit" : "the index";
  const lost = changes[0].staged ? "staged and on disk" : "on disk";
  const description = !one
    ? `Chord ${[restored && `restores ${files(restored)} from ${source}`, untracked && `moves ${files(untracked)} that git does not track to the Trash`].filter(Boolean).join(" and ")}.${restored ? ` The changes ${lost} are lost.` : ""}`
    : goesToTrash(one)
      ? `Git holds no earlier version of ${fileName(one.path)}, so Chord moves it to the Trash.`
      : `Chord restores ${hunk ? "this hunk of " : ""}${fileName(one.path)} from ${source}. The change ${lost} is lost.`;
  return (
    <ConfirmSheet
      title={hunk ? "Discard Hunk?" : one ? "Discard Changes?" : `Discard ${changes.length} Files?`}
      description={description}
      detail={one?.path}
      confirmLabel={one && goesToTrash(one) ? "Move to Trash" : "Discard"}
      onCancel={() => closeSheet("discardChanges")}
      onConfirm={() => void confirmDiscard()}
    />
  );
}
