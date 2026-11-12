// The sheet that asks before a worktree git refused to remove is removed with force.
import { ConfirmSheet } from "../../ui/ConfirmSheet";
import { cancelForceRemoval, confirmForceRemoval, forceRemoval } from "./store";

export function ForceRemovalPopup() {
  const pending = forceRemoval.use();
  if (!pending) return null;
  return (
    <ConfirmSheet
      title="Force Delete?"
      description={`Deleting ${pending.branch ?? pending.path} anyway discards its uncommitted changes.`}
      detail={pending.reason}
      confirmLabel="Delete"
      onCancel={cancelForceRemoval}
      onConfirm={confirmForceRemoval}
    />
  );
}
