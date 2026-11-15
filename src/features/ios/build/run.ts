// Rules for a worktree's build: how Rust's events change it, how the card's marker reads it, and where an error's file lies.
import type { BuildEvent_Deserialize, BuildIssue_Deserialize } from "../../../ipc/bindings";
import { appendToLog, emptyLog, type Log } from "./log.ts";

export type Build = {
  phase: "preparing" | "waiting" | "building" | "installing" | "launching" | "running" | "exited" | "succeeded" | "failed" | "stopped";
  startedAt: number;
  /** The first errors, and how many more the build printed. */
  issues: BuildIssue_Deserialize[];
  more: number;
  /** The user saw the success or the stop, so its marker goes. */
  endSeen: boolean;
};

/** A worktree has no build until Rust reports its first phase. */
export type BuildAndLog = { build: Build | undefined; log: Log };

export const noBuild: BuildAndLog = { build: undefined, log: emptyLog };

// `startedAt` is when the build began compiling: the time Preparing and Waiting took does not count.
export function appliedEvent(state: BuildAndLog, event: BuildEvent_Deserialize, now: number): BuildAndLog {
  const build = state.build ?? { phase: "preparing", startedAt: now, issues: [], more: 0, endSeen: false };
  switch (event.kind) {
    case "phase":
      return { ...state, build: { ...build, phase: event.phase, startedAt: event.phase === "building" ? now : build.startedAt } };
    case "log":
      return { ...state, log: appendToLog(state.log, event.text) };
    case "succeeded":
      return { ...state, build: { ...build, phase: "succeeded" } };
    case "exited":
      return { ...state, build: { ...build, phase: "exited" } };
    case "stopped":
      return { ...state, build: { ...build, phase: "stopped" } };
    case "failed":
      return { ...state, build: { ...build, phase: "failed", issues: event.issues, more: event.more } };
  }
}

// From the start of a build until the app launches or the build ends: the run control shows stop.
export const isStarting = (build: Build | undefined): boolean =>
  build?.phase === "preparing" || build?.phase === "waiting" || build?.phase === "building" || build?.phase === "installing" || build?.phase === "launching";

export const isRunning = (build: Build | undefined): boolean => build?.phase === "running";

export const errorCount = (build: Build): number => build.issues.length + build.more;

export type Marker = { text: string; tone: "busy" | "done" | "stopped" | "failed" };

// Marks a success or a stop as seen, so it leaves the card; any other build stays as it is.
export const seenEnd = (build: Build): Build =>
  (build.phase === "succeeded" || build.phase === "stopped") && !build.endSeen ? { ...build, endSeen: true } : build;

// The marker on the worktree's card: the last build's phase, until the next build starts; a seen success or stop shows none.
export function markerText(build: Build): Marker | null {
  switch (build.phase) {
    case "preparing":
    case "waiting":
    case "building":
    case "installing":
    case "launching":
      return { text: "Building", tone: "busy" };
    case "running":
      return { text: "Running", tone: "busy" };
    case "exited":
      return null;
    case "succeeded":
      return build.endSeen ? null : { text: "Build succeeded", tone: "done" };
    case "stopped":
      return build.endSeen ? null : { text: "Build stopped", tone: "stopped" };
    case "failed":
      return { text: "Build failed", tone: "failed" };
  }
}

// An error's file as a path inside the worktree, or null for a file outside it.
export function pathInWorktree(worktree: string, file: string): string | null {
  return file.startsWith(`${worktree}/`) ? file.slice(worktree.length + 1) : null;
}
