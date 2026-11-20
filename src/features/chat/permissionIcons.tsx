// The icon of each permission mode a harness names; a mode Chord does not know takes the shield.
import type { ComponentType } from "react";
import { BlockIcon, EyeIcon, FolderIcon, PencilIcon, PlanIcon, ShieldCheckIcon, ShieldIcon, UnlockIcon } from "../../ui/icons";

const icons: Record<string, ComponentType> = {
  "Read Only": EyeIcon,
  Workspace: () => <FolderIcon size={16} />,
  "Full Access": UnlockIcon,
  Manual: ShieldIcon,
  "Accept Edits": PencilIcon,
  Plan: PlanIcon,
  Auto: ShieldCheckIcon,
  "Bypass Permissions": UnlockIcon,
  "Don't Ask": BlockIcon,
};

// Codex names a policy pair off its modes "<sandbox>, <approval>"; the sandbox picks the icon.
export function PermissionIcon({ mode }: { mode: string }) {
  const Icon = icons[mode.split(",")[0]] ?? ShieldIcon;
  return <Icon />;
}
