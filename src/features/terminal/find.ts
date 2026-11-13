// Find in the terminal: which worktrees' terminals have the find bar open and what they search for.
import { findStore } from "../../ui/find";

export const { finds: terminalFinds, openFind: openTerminalFind, setFindQuery: setTerminalFindQuery, closeFind: closeTerminalFind } = findStore();

