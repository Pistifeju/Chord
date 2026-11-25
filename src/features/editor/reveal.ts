import { externalStore } from "../../ui/externalStore";

const key = (worktree: string, path: string) => `${worktree}\0${path}`;

export const lineRequests = externalStore<Record<string, number>>({});

export const requestLine = (worktree: string, path: string, line: number) => lineRequests.set({ ...lineRequests.get(), [key(worktree, path)]: line });

export const requestedLine = (all: Record<string, number>, worktree: string, path: string): number | undefined => all[key(worktree, path)];

export function takeLine(worktree: string, path: string) {
  const { [key(worktree, path)]: _taken, ...rest } = lineRequests.get();
  lineRequests.set(rest);
}
