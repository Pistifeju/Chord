// The Files view's commands for the commands table: show or hide the view, open a file by name, and the tree's right-click actions on an entry's relative path.
import type { AppCommand } from "../../app/commands";
import { onTarget } from "../../app/commandTarget";
import { settingsOpen } from "../settings/store";
import { toggleNavigator } from "../sidebar/store";
import { openWorktree, projects } from "../worktrees/store";
import { fail } from "../../ui/alert";
import { openFileByName, requestTrash, revealFile, startNew, startRename } from "./store";
import { joinPath } from "./paths";

// The tree row that holds the keyboard focus, so ⌘⌫ never acts on a row the user did not pick.
const focusedEntry = (key: "path" | "folder" = "path") =>
  (document.activeElement as HTMLElement | null)?.closest<HTMLElement>(".files-row")?.dataset[key];

// Runs `act` on the open worktree and the target entry; a menu passes the target, a shortcut takes the focused row's `key`.
const onEntry = (act: (worktree: string, path: string) => void, key: "path" | "folder" = "path") => (target?: string) => {
  const worktree = projects.get().selected;
  const path = target ?? focusedEntry(key);
  if (worktree && path !== undefined) act(worktree, path);
};

// The Files view's filter field while the focus is anywhere in the sidebar showing it, its navigator included.
const focusedFilter = () =>
  document.activeElement?.closest(".sidebar")?.querySelector<HTMLInputElement>(".files-filter input");

const copy = (text: string) => void navigator.clipboard.writeText(text).catch(fail);

export const filesCommands: AppCommand[] = [
  { id: "files.toggle", title: "Show or Hide Files", shortcut: { key: "e", cmd: true, shift: true }, ...onTarget(openWorktree, () => toggleNavigator("files")) },
  { id: "files.openByName", title: "Open File by Name", shortcut: { key: "o", cmd: true, shift: true }, ...onTarget(openWorktree, (worktree) => void openFileByName(worktree)) },
  { id: "files.find", title: "Filter Files", shortcut: { key: "f", cmd: true }, ...onTarget(focusedFilter, (field) => field.select()) },
  { id: "files.newFile", title: "New File", run: onEntry((worktree, folder) => startNew(worktree, folder, "newFile"), "folder") },
  { id: "files.newFolder", title: "New Folder", run: onEntry((worktree, folder) => startNew(worktree, folder, "newFolder"), "folder") },
  { id: "files.rename", title: "Rename", run: onEntry(startRename) },
  { id: "files.copyPath", title: "Copy Path", run: onEntry((worktree, path) => copy(joinPath(worktree, path))) },
  { id: "files.copyRelativePath", title: "Copy Relative Path", run: onEntry((_, path) => copy(path)) },
  { id: "files.reveal", title: "Reveal in Finder", run: onEntry(revealFile) },
  {
    id: "files.trash",
    title: "Move to Trash",
    shortcut: { key: "backspace", cmd: true },
    when: () => !settingsOpen.get() && focusedEntry() !== undefined,
    run: onEntry(requestTrash),
  },
];
