// Rules for a worktree's build: how Rust's events change it, how the card's marker reads it, and where an error's file lies and how its place reads.
import type { BuildEvent_Deserialize, BuildIssue_Deserialize } from "../../../ipc/bindings";
import type { BottomViewSlot } from "../../bottompanel/BottomPanel";
import type { BottomView } from "../../../ipc/bindings";
import { appendToConsole, emptyConsole, type ConsoleLine } from "./consoleLog.ts";
import { appendToLog, emptyLog, type Log } from "./log.ts";

export type Build = {
  phase: "preparing" | "waiting" | "building" | "installing" | "launching" | "running" | "exited" | "succeeded" | "failed" | "stopped";
  startedAt: number;
  /** The first errors, and how many more the build printed. */
  issues: BuildIssue_Deserialize[];
  more: number;
  /** The user saw the success or the stop, so its marker goes. */
  endSeen: boolean;
  /** A ⌘B that came while the app ran: the app and its console go on, and this build shows beside them. */
  side?: Build;
};

/** A worktree has no build until Rust reports its first phase. */
export type BuildAndLog = { build: Build | undefined; log: Log; console: ConsoleLine[] };

export const noBuild: BuildAndLog = { build: undefined, log: emptyLog, console: emptyConsole };

// `startedAt` is when the build began compiling: the time Preparing and Waiting took does not count.
export function appliedEvent(state: BuildAndLog, event: BuildEvent_Deserialize, now: number): BuildAndLog {
  const build = state.build ?? { phase: "preparing", startedAt: now, issues: [], more: 0, endSeen: false };
  switch (event.kind) {
    case "phase":
      return { ...state, build: { ...build, phase: event.phase, startedAt: event.phase === "building" ? now : build.startedAt } };
    case "log":
      return { ...state, log: appendToLog(state.log, event.text) };
    case "console":
      return { ...state, console: appendToConsole(state.console, event.text, event.tone) };
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

// A build beside the running app: its phases and errors, while the app's phase stays.
export function appliedSideEvent(state: BuildAndLog, event: BuildEvent_Deserialize, now: number): BuildAndLog {
  if (!state.build) return state;
  const side = state.build.side ?? { phase: "preparing", startedAt: now, issues: [], more: 0, endSeen: true };
  switch (event.kind) {
    case "phase":
      return { ...state, build: { ...state.build, side: { ...side, phase: event.phase, startedAt: event.phase === "building" ? now : side.startedAt } } };
    case "log":
      return { ...state, log: appendToLog(state.log, event.text) };
    case "succeeded":
    case "stopped":
      return { ...state, build: { ...state.build, side: { ...side, phase: event.kind } } };
    case "failed":
      return { ...state, build: { ...state.build, side: { ...side, phase: "failed", issues: event.issues, more: event.more } } };
    default:
      return state;
  }
}

// From the start of a build until the app launches or the build ends: the run control shows stop.
export const isStarting = (build: Build | undefined): boolean =>
  build?.phase === "preparing" || build?.phase === "waiting" || build?.phase === "building" || build?.phase === "installing" || build?.phase === "launching";

export const isRunning = (build: Build | undefined): boolean => build?.phase === "running";

// Console has output only once the app has run; before that Build is the view with something to show.
export const viewToShow = (build: Build | undefined): BottomViewSlot["id"] => (build?.phase === "running" || build?.phase === "exited" ? "console" : "build");

// A launching app takes the Build column over; any other event leaves the view as it is.
export const viewAfterEvent = (view: BottomView, event: BuildEvent_Deserialize): BottomView =>
  view === "build" && event.kind === "phase" && event.phase === "running" ? "console" : view;

export const errorCount = (build: Build): number => build.issues.length + build.more;

export type Marker = { text: string; tone: "busy" | "done" | "stopped" | "failed" };

// Marks a success, a run past its build, or a stop as seen, so it leaves the card; any other build stays as it is.
export const seenEnd = (build: Build): Build =>
  ["succeeded", "installing", "launching", "stopped"].includes(build.phase) && !build.endSeen ? { ...build, endSeen: true } : build;

// The marker on the worktree's card: the last build's phase, until the next build starts; a seen success or stop shows none.
export function markerText(build: Build): Marker | null {
  if (build.side && isStarting(build.side)) return { text: "Building", tone: "busy" };
  if (build.side?.phase === "failed") return { text: "Build failed", tone: "failed" };
  switch (build.phase) {
    case "preparing":
    case "waiting":
    case "building":
      return { text: "Building", tone: "busy" };
    case "installing":
    case "launching":
    case "succeeded":
      return build.endSeen ? null : { text: "Build succeeded", tone: "done" };
    case "running":
      return { text: "Running", tone: "busy" };
    case "exited":
      return null;
    case "stopped":
      return build.endSeen ? null : { text: "Build stopped", tone: "stopped" };
    case "failed":
      return { text: "Build failed", tone: "failed" };
  }
}

/** "LoginView.swift:42", or the file's name alone without a line; null without a file. */
export function issuePlace(issue: BuildIssue_Deserialize): string | null {
  const name = issue.file?.split("/").pop();
  if (!name) return null;
  return issue.line ? `${name}:${issue.line}` : name;
}

// An error's file as a path inside the worktree, or null for a file outside it.
export function pathInWorktree(worktree: string, file: string): string | null {
  return file.startsWith(`${worktree}/`) ? file.slice(worktree.length + 1) : null;
}
