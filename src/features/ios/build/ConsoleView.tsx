import { useMemo } from "react";
import { LogView } from "./LogView";
import { closeConsoleFind, consoleFinds, setConsoleFindQuery } from "./find";
import { builds, consoles } from "./store";

export function ConsoleView({ worktree }: { worktree: string }) {
  const build = builds.usePart((all) => all[worktree]);
  const output = consoles.usePart((all) => all[worktree]);
  const find = consoleFinds.usePart((all) => all[worktree]);
  const texts = useMemo(() => (output ?? []).map((line) => line.text), [output]);
  if (texts.length === 0) return <div className="log-part log-empty">No app output yet</div>;
  return (
    <LogView
      view="console"
      label="Console"
      lines={texts}
      toneAt={(line) => output![line].tone}
      restartKey={build?.startedAt}
      atStart={false}
      find={find}
      findLabel="Find in Console"
      onFindQuery={(query) => setConsoleFindQuery(worktree, query)}
      onFindClose={() => closeConsoleFind(worktree)}
    />
  );
}
