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

export function PermissionIcon({ mode }: { mode: string }) {
  const Icon = icons[mode.split(",")[0]] ?? ShieldIcon;
  return <Icon />;
}
