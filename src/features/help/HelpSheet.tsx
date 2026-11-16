// The Help sheet: what a project, a worktree and a session are, what each agent status means, and the shortcuts used most.
import type { Status } from "../../ipc/bindings";
import { commandTitles, shortcutLabel, tooltip } from "../../app/commands";
import { closeSheet, sheet } from "../../app/sheet";
import { CloseIcon } from "../../ui/icons";
import { openShortcuts } from "../settings/store";
import { StatusMark } from "../status/StatusMark";
import { statusLabel } from "../status/status";
import "./help.css";

const terms = [
  { name: "Project", text: "A git repository you added." },
  { name: "Worktree", text: "A checkout of the project on its own branch. Give each task its own, and its agents never touch another task's files." },
  { name: "Session", text: "An agent chat inside a worktree. A worktree holds as many as you open." },
];

const statuses: { status: Status; text: string }[] = [
  { status: { kind: "running" }, text: "The agent is at work." },
  { status: { kind: "waiting" }, text: "The agent asked you something." },
  { status: { kind: "done" }, text: "The agent finished. The word stays until you open the session." },
  { status: { kind: "error" }, text: "The agent stopped on an error." },
  { status: { kind: "exited" }, text: "The agent's process ended." },
];

const shortcuts = ["worktree.new", "session.new", "sidebar.toggle", "files.toggle", "terminal.toggle", "settings.open"];

function showAllShortcuts() {
  closeSheet("help");
  openShortcuts();
}

export function HelpSheet() {
  const open = sheet.use()?.kind === "help";
  if (!open) return null;
  const close = () => closeSheet("help");
  return (
    <div className="sheet-backdrop" onMouseDown={close}>
      <div className="sheet help" role="dialog" aria-modal="true" aria-label="Help" onMouseDown={(e) => e.stopPropagation()}>
        <header className="sheet-header">
          <h2 className="sheet-title">Help</h2>
          <button type="button" className="icon-button" aria-label="Close" title={tooltip("sheet.close")} onClick={close}>
            <CloseIcon />
          </button>
        </header>
        <section className="help-section" aria-label="Basics">
          <h3 className="field-label">Basics</h3>
          <dl className="help-list">
            {terms.map(({ name, text }) => (
              <div key={name} className="help-term">
                <dt>{name}</dt>
                <dd>{text}</dd>
              </div>
            ))}
          </dl>
        </section>
        <section className="help-section" aria-label="Statuses">
          <h3 className="field-label">Statuses</h3>
          <dl className="help-list">
            {statuses.map(({ status, text }) => (
              <div key={status.kind} className="help-term">
                <dt className="help-status" data-status={status.kind}>
                  <StatusMark kind={status.kind} />
                  {statusLabel(status)}
                </dt>
                <dd>{text}</dd>
              </div>
            ))}
          </dl>
        </section>
        <section className="help-section" aria-label="Shortcuts">
          <h3 className="field-label">Shortcuts</h3>
          <dl className="help-list">
            {shortcuts.map((id) => {
              const keys = shortcutLabel(id);
              if (!keys) return null;
              return (
                <div key={id} className="help-shortcut">
                  <dt>{commandTitles.find((c) => c.id === id)?.title}</dt>
                  <dd>{keys}</dd>
                </div>
              );
            })}
          </dl>
        </section>
        <footer className="sheet-footer">
          <button type="button" className="button-secondary" onClick={showAllShortcuts}>
            All Shortcuts
          </button>
        </footer>
      </div>
    </div>
  );
}
