import { sheet, sheetOf } from "../../app/sheet";
import { tooltip } from "../../app/commands";
import { CloseIcon } from "../../ui/icons";
import { closeNewBranch, createBranch, newBranch, setNewBranchName } from "./newBranch";
import { branchName } from "./projects";

export function NewBranchPopup() {
  const open = sheetOf(sheet.use(), "newBranch");
  const form = newBranch.use();
  if (!open || !form) return null;
  const typed = branchName(form.name);

  return (
    <div className="sheet-backdrop" onMouseDown={closeNewBranch}>
      <form
        className="sheet"
        role="dialog"
        aria-modal="true"
        aria-label="New Branch"
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault();
          void createBranch();
        }}
      >
        <header className="sheet-header">
          <h2 className="sheet-title">New Branch</h2>
          <button type="button" className="icon-button" aria-label="Cancel" title={tooltip("sheet.close")} onClick={closeNewBranch}>
            <CloseIcon />
          </button>
        </header>
        <label className="field">
          <span className="field-label">Branch name</span>
          <input className="input" autoFocus spellCheck={false} autoCapitalize="off" autoCorrect="off" value={form.name} readOnly={form.creating} onChange={(e) => void setNewBranchName(e.target.value)} />
          <span className="field-hint" data-error={form.problem ? true : undefined}>
            {form.problem ?? (typed || (form.from ? `Starts from ${form.from}` : "Starts from this commit"))}
          </span>
        </label>
        <footer className="sheet-footer">
          <button type="submit" className="button-primary" disabled={!typed || !!form.problem} aria-busy={form.creating}>
            <span className="button-label">Create</span>
            {form.creating && <span className="spinner" aria-label="Creating" />}
          </button>
        </footer>
      </form>
    </div>
  );
}
