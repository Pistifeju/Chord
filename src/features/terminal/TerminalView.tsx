// The worktree's terminal in the bottom panel: a header with its title and Hide button, then the xterm.
import { useEffect, useRef } from "react";
import "@xterm/xterm/css/xterm.css";
import { tooltip } from "../../app/commands";
import { CloseIcon } from "../../ui/icons";
import { hideTerminal, showTerminal } from "./store";
import "./terminal.css";

export function TerminalView({ worktree }: { worktree: string }) {
  const container = useRef<HTMLDivElement>(null);
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
    </div>
  );
}
