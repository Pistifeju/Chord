// The worktree's terminal in the bottom panel: a header with its title and Hide button, then the xterm and its find bar.
import { useEffect, useRef } from "react";
import { tooltip } from "../../app/commands";
import { CloseIcon } from "../../ui/icons";
import { terminalFinds } from "./find";
import { hideTerminal, showTerminal } from "./store";
import { TerminalFind } from "./TerminalFind";

export function TerminalView({ worktree }: { worktree: string }) {
  const container = useRef<HTMLDivElement>(null);
  const find = terminalFinds.usePart((all) => all[worktree]);
  useEffect(() => showTerminal(worktree, container.current!), [worktree]);
  return (
    <div className="terminal-part">
      <div className="terminal-header">
        <span className="terminal-title">Terminal</span>
        <button type="button" className="icon-button terminal-hide" title={tooltip("terminal.toggle")} aria-label="Hide Terminal" onClick={() => hideTerminal(worktree)}>
          <CloseIcon />
        </button>
      </div>
      <div ref={container} className="terminal-body" />
      {find && <TerminalFind worktree={worktree} find={find} />}
    </div>
  );
}
