// The New Worktree sheet: project picker, branch name with its live git check, Create.
import type { FormEvent } from "react";
import { tooltip } from "../../app/commands";
import { sheet } from "../../app/sheet";
import { CloseIcon } from "../../ui/icons";
import { branchName, canCreate } from "./projects";
import { closeNewWorktree, createWorktree, newWorktree, setBranch, setProject } from "./newWorktree";
import { projects } from "./store";

export function NewWorktreePopup() {
  const open = sheet.use()?.kind === "newWorktree";
  const { project, branch, problem, checked, creating, createError } = newWorktree.use();
  const { list } = projects.use();
  if (!open) return null;
  const submit = (e: FormEvent) => {
    e.preventDefault();
    void createWorktree();
  };
  const ready = canCreate({ project, branch, checked, problem, creating });
  return (
    <div className="sheet-backdrop" onMouseDown={closeNewWorktree}>
      <form className="sheet" role="dialog" aria-modal="true" aria-label="New Worktree" onMouseDown={(e) => e.stopPropagation()} onSubmit={submit}>
        <header className="sheet-header">
          <h2 className="sheet-title">New Worktree</h2>
          <button type="button" className="icon-button" aria-label="Cancel" title={tooltip("sheet.close")} onClick={closeNewWorktree}>
            <CloseIcon />
          </button>
        </header>
        <label className="field">
          <span className="field-label">Project</span>
          <select className="input" value={project ?? ""} onChange={(e) => setProject(e.target.value)}>
            {list.map((p) => (
              <option key={p.path} value={p.path}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span className="field-label">Branch name</span>
          <input
            className="input"
            autoFocus
            spellCheck={false}
            autoCapitalize="off"
            autoCorrect="off"
            value={branch}
            onChange={(e) => setBranch(e.target.value)}
          />
          <span className="field-hint" data-error={createError || problem ? true : undefined}>
            {createError ?? problem ?? branchName(branch)}
          </span>
        </label>
        <footer className="sheet-footer">
          <button type="submit" className="button-primary" disabled={!ready && !creating} aria-busy={creating}>
            <span className="button-label">Create</span>
            {creating && <span className="spinner" aria-label="Creating" />}
          </button>
        </footer>
      </form>
    </div>
  );
}
