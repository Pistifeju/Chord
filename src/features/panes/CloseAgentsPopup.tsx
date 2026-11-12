// The sheet that confirms closing a pane or tab while its agent works.
import { sheet, sheetOf } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { cancelClose, confirmClose } from "./store";

export function CloseAgentsPopup() {
  const open = sheetOf(sheet.use(), "closeAgents");
  if (!open) return null;
  const what = open.target === "pane" ? "Pane" : "Session";
  return (
    <ConfirmSheet
      title={`Close ${what}?`}
      description="The agent is still working. Closing stops it."
      confirmLabel={`Close ${what}`}
      onCancel={cancelClose}
      onConfirm={() => confirmClose(open)}
    />
  );
}
