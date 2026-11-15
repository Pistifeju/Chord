// Find in the build log and in the console: which worktrees' views have the find bar open and what they search for.
import { findStore } from "../../../ui/find";

export const { finds: buildFinds, openFind: openBuildFind, setFindQuery: setBuildFindQuery, closeFind: closeBuildFind } = findStore();
export const { finds: consoleFinds, openFind: openConsoleFind, setFindQuery: setConsoleFindQuery, closeFind: closeConsoleFind } = findStore();

/** The find functions of each log view, so a command picks them by the view that has the focus. */
export const findsOf = {
  build: { finds: buildFinds, open: openBuildFind, close: closeBuildFind },
  console: { finds: consoleFinds, open: openConsoleFind, close: closeConsoleFind },
};
