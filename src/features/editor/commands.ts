// The editor's commands for the commands table: find in the open file.
import type { AppCommand } from "../../app/commands";
import { onTarget } from "../../app/commandTarget";
import { editorFinds, endEditorFind, openEditorFind } from "./find";

// The editor that holds the focus, in its text or its find bar; the image view has no `data-find`.
const focusedEditor = (): string | null => document.activeElement?.closest<HTMLElement>(".editor")?.dataset.find ?? null;

const findingEditor = (): string | null => {
  const editor = focusedEditor();
  return editor && editorFinds.get()[editor] ? editor : null;
};

export const editorCommands: AppCommand[] = [
  { id: "editor.find", title: "Find in File", shortcut: { key: "f", cmd: true }, ...onTarget(focusedEditor, openEditorFind) },
  { id: "editor.findClose", title: "Close Find in File", shortcut: { key: "escape" }, ...onTarget(findingEditor, endEditorFind) },
];
