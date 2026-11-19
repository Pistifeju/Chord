// The parts of a changed file's row in the Changes view, staged, unstaged or committed: its name, its folder, the lines it added and removed, and its status letter.
import type { ChangeStatus } from "../../ipc/bindings";
import { fileName, parentFolder } from "../files/paths";
import { statusLetters, statusWord } from "./changes";

export function ChangeParts({ path, status, added, removed }: { path: string; status: ChangeStatus; added?: number | null; removed?: number | null }) {
  const folder = parentFolder(path);
  return (
    <>
      <span className="files-name changes-name" data-status={status}>
        {fileName(path)}
      </span>
      {folder && <span className="changes-folder">{folder}</span>}
      <span className="changes-lines">
        {!!added && <span className="changes-added">+{added}</span>}
        {!!removed && <span className="changes-removed">−{removed}</span>}
      </span>
      <span className="changes-status" data-status={status} title={statusWord(status)}>
        {statusLetters[status]}
      </span>
    </>
  );
}
