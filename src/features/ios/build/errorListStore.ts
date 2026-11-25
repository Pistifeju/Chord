import { externalStore } from "../../../ui/externalStore";

export const errorList = externalStore<string | null>(null);

export const toggleErrorList = (worktree: string) => errorList.set(errorList.get() === worktree ? null : worktree);

export const closeErrorList = () => errorList.set(null);
