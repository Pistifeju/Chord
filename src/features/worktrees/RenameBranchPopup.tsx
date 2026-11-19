// The Rename Branch sheet: the new name with its live git check, Rename.
import { sheet, sheetOf } from "../../app/sheet";
import { tooltip } from "../../app/commands";
import { CloseIcon } from "../../ui/icons";
import { branchName } from "./projects";
import { cancelRename, finishRename, rename, setRenameName } from "./rename";

export function RenameBranchPopup() {
  const open = sheetOf(sheet.use(), "renameBranch");
  const form = rename.use();
  if (!open || !form || form.path !== null) return null;
  const typed = branchName(form.name);

  return (
    <div className="sheet-backdrop" onMouseDown={cancelRename}>
      <form
        className="sheet"
        role="dialog"
        aria-modal="true"
        aria-label="Rename Branch"
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault();
          void finishRename();
        }}
      >
        <header className="sheet-header">
          <h2 className="sheet-title">Rename Branch</h2>
          <button type="button" className="icon-button" aria-label="Cancel" title={tooltip("sheet.close")} onClick={cancelRename}>
            <CloseIcon />
          </button>
        </header>
        <label className="field">
          <span className="field-label">Branch name</span>
          <input className="input" autoFocus spellCheck={false} autoCapitalize="off" autoCorrect="off" value={form.name} readOnly={form.renaming} onChange={(e) => void setRenameName(e.target.value)} onFocus={(e) => e.currentTarget.select()} />
          <span className="field-hint" data-error={form.problem ? true : undefined}>
            {form.problem ?? typed}
          </span>
        </label>
        <footer className="sheet-footer">
          <button type="submit" className="button-primary" disabled={!typed || !!form.problem} aria-busy={form.renaming}>
            <span className="button-label">Rename</span>
            {form.renaming && <span className="spinner" aria-label="Renaming" />}
          </button>
        </footer>
      </form>
    </div>
  );
}
