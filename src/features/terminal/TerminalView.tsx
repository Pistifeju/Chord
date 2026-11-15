// The worktree's terminal in the bottom panel: the xterm and its find bar.
import { useEffect, useRef } from "react";
import { terminalFinds } from "./find";
import { showTerminal } from "./store";
import { TerminalFind } from "./TerminalFind";

export function TerminalView({ worktree }: { worktree: string }) {
  const container = useRef<HTMLDivElement>(null);
  const find = terminalFinds.usePart((all) => all[worktree]);
  useEffect(() => showTerminal(worktree, container.current!), [worktree]);
  return (
    <div className="terminal-part">
      <div ref={container} className="terminal-body" />
      {find && <TerminalFind worktree={worktree} find={find} />}
    </div>
  );
}
