// The sheet that confirms closing a tab while its agent works.
import { sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { cancelClose, confirmClose } from "./store";

export function CloseAgentsPopup() {
  const open = sheetOf(sheet.use(), "closeAgents");
  if (!open) return null;
  return (
    <ConfirmSheet
      title="Close Session?"
      description="The agent is still working. Closing stops it."
      confirmLabel="Close Session"
      onCancel={cancelClose}
      onConfirm={() => confirmClose(open)}
    />
  );
}
