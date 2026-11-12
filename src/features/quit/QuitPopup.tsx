// The sheet that confirms a quit while agents are working.
import { sheet } from "../../app/sheet";
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { cancelQuit, confirmQuit } from "./store";

export function QuitPopup() {
  if (sheet.use()?.kind !== "quit") return null;
  return (
    <ConfirmSheet
      title="Quit Chord?"
      description="Agents are still working. They stop, and come back in their sessions on launch."
      confirmLabel="Quit"
      onCancel={cancelQuit}
      onConfirm={confirmQuit}
    />
  );
}
