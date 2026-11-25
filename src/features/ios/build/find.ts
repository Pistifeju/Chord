import { findStore } from "../../../ui/find";

export const { finds: buildFinds, openFind: openBuildFind, setFindQuery: setBuildFindQuery, closeFind: closeBuildFind } = findStore();
export const { finds: consoleFinds, openFind: openConsoleFind, setFindQuery: setConsoleFindQuery, closeFind: closeConsoleFind } = findStore();

export const findsOf = {
  build: { finds: buildFinds, open: openBuildFind, close: closeBuildFind },
  console: { finds: consoleFinds, open: openConsoleFind, close: closeConsoleFind },
};
