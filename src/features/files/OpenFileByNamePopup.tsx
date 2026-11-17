// The Open File by Name sheet: a field for the name, the worktree's matching files under it; Return or a click opens one.
import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { FileIcon } from "../../ui/icons";
import { PickerSheet } from "../../ui/PickerSheet";
import { matchingFiles } from "./openByName";
import { pickFileByName } from "../editor/showing";
import { namedFiles } from "./store";

const samePath = (path: string) => path;

function FileRow({ path }: { path: string }) {
  const slash = path.lastIndexOf("/");
  return (
    <>
      <FileIcon />
      <span className="picker-sheet-name">{path.slice(slash + 1)}</span>
      {slash !== -1 && <span className="picker-sheet-folder">{path.slice(0, slash)}</span>}
    </>
  );
}

export function OpenFileByNamePopup() {
  const open = sheetOf(sheet.use(), "openFileByName");
  const files = namedFiles.use();
  if (!open) return null;
  return (
    <PickerSheet
      key={open.worktree}
      label="Open File by Name"
      placeholder="File name"
      fieldLabel="File Name"
      items={files}
      matching={matchingFiles}
      keyOf={samePath}
      titleOf={samePath}
      row={(path) => <FileRow path={path} />}
      onPick={(path) => pickFileByName(open.worktree, path)}
      onClose={() => closeSheet("openFileByName")}
    />
  );
}
