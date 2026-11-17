// The preview picker sheet: a search field over the worktree's `#Preview`s by file; Return or a click shows one.
import { closeSheet, sheet, sheetOf } from "../../../app/sheet";
import type { PreviewPick } from "../../../ipc/bindings";
import { PickerSheet } from "../../../ui/PickerSheet";
import { matchingPreviews, previewName } from "./preview";
import { choosePreview, previewList } from "./store";

const keyOf = (pick: PreviewPick) => `${pick.file}#${pick.index}`;
const titleOf = (pick: PreviewPick) => `${previewName(pick)} · ${pick.file}`;

export function PreviewPickerPopup() {
  const open = sheetOf(sheet.use(), "previewPicker");
  const all = previewList.use();
  if (!open) return null;
  return (
    <PickerSheet
      key={open.worktree}
      label="Choose Preview"
      placeholder="Preview name"
      fieldLabel="Preview Name"
      items={all}
      matching={matchingPreviews}
      keyOf={keyOf}
      titleOf={titleOf}
      row={(pick) => (
        <>
          <span className="picker-sheet-name">{previewName(pick)}</span>
          <span className="picker-sheet-folder">{pick.file}</span>
        </>
      )}
      noItems="No previews in this worktree"
      onPick={(pick) => choosePreview(open.worktree, pick)}
      onClose={() => closeSheet("previewPicker")}
    />
  );
}
