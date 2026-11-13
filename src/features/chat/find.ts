// Find in the transcript: which panes have the find bar open and what they search for.
import { findStore } from "../../ui/find";

export type { Find } from "../../ui/find";

export const { finds, openFind, setFindQuery, closeFind } = findStore();
