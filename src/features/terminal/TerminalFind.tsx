import { useEffect, useState } from "react";
import { tooltip } from "../../app/commands";
import { FindBar } from "../../ui/FindBar";
import type { Find } from "../../ui/find";
import { closeTerminalFind, setTerminalFindQuery } from "./find";
import { endTerminalFind, findInTerminal, focusTerminal, onTerminalFindResults } from "./store";

export function TerminalFind({ worktree, find }: { worktree: string; find: Find }) {
  const [results, setResults] = useState({ current: 0, count: 0 });

  useEffect(() => {
    const listening = onTerminalFindResults(worktree, ({ resultIndex, resultCount }) =>
      setResults({ current: Math.max(0, resultIndex), count: resultCount }),
    );
    return () => {
      listening?.dispose();
      endTerminalFind(worktree);
    };
  }, [worktree]);

  useEffect(() => {
    if (!find.query) setResults({ current: 0, count: 0 });
    findInTerminal(worktree, find.query, 0);
  }, [worktree, find.query]);

  return (
    <FindBar
      query={find.query}
      focus={find.focus}
      current={results.current}
      count={results.count}
      label="Find in Terminal"
      closeTitle={tooltip("terminal.findClose")}
      onQuery={(query) => setTerminalFindQuery(worktree, query)}
      onStep={(by) => findInTerminal(worktree, find.query, by)}
      onClose={(byButton) => {
        closeTerminalFind(worktree);
        if (byButton) focusTerminal(worktree);
      }}
    />
  );
}
