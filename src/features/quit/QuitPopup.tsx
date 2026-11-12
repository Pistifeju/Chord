// The sheet that confirms a quit while agents are working.
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { cancelQuit, confirmQuit, quitAsked } from "./store";

export function QuitPopup() {
  if (!quitAsked.use()) return null;
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
