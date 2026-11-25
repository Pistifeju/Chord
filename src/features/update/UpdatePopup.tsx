import { sheet } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { cancelUpdate, confirmUpdate } from "./store";

export function UpdatePopup() {
  if (sheet.use()?.kind !== "update") return null;
  return (
    <ConfirmSheet
      title="Update Chord?"
      description="Agents are still working. They stop, and come back in their sessions once Chord restarts."
      confirmLabel="Update"
      onCancel={cancelUpdate}
      onConfirm={confirmUpdate}
    />
  );
}
