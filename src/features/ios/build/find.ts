// Find in the build log: which worktrees' logs have the find bar open and what they search for.
import { findStore } from "../../../ui/find";

export const { finds: buildFinds, openFind: openBuildFind, setFindQuery: setBuildFindQuery, closeFind: closeBuildFind } = findStore();
