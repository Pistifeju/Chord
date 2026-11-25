import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { confirmRemoveProject } from "./store";

export function RemoveProjectPopup() {
  const pending = sheetOf(sheet.use(), "removeProject");
  if (!pending) return null;
  return (
    <ConfirmSheet
      title="Remove Project?"
      description={`An agent in ${pending.name} is still working. Removing the project stops it.`}
      confirmLabel="Remove Project"
      onCancel={() => closeSheet("removeProject")}
      onConfirm={confirmRemoveProject}
    />
  );
}
