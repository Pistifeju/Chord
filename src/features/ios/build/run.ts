// Rules for a worktree's build: how Rust's events change it, how the card's marker reads it, and where an error's file lies and how its place reads.
import type { BuildEvent_Deserialize, BuildIssue_Deserialize } from "../../../ipc/bindings";
import type { BottomViewSlot } from "../../bottompanel/BottomPanel";
import type { BottomView } from "../../../ipc/bindings";
import { appendToConsole, emptyConsole, type ConsoleLine } from "./consoleLog.ts";
import { appendToLog, emptyLog, type Log } from "./log.ts";
import { fileName } from "../../files/paths.ts";

export type Build = {
  phase: "preparing" | "waiting" | "building" | "installing" | "launching" | "running" | "exited" | "succeeded" | "failed" | "stopped";
  startedAt: number;
  /** The first errors, and how many more the build printed. */
  issues: BuildIssue_Deserialize[];
  more: number;
  /** The user saw the success or the stop, so its marker goes. */
  endSeen: boolean;
  /** When it succeeded, failed, stopped or its app exited. */
  endedAt?: number;
  /** What the log says it does now, "Compiling ChordKit"; none before the first step. */
  step?: string;
  /** A ⌘B that came while the app ran: the app and its console go on, and this build shows beside them. */
  side?: Build;
};

/** A worktree has no build until Rust reports its first phase. */
export type BuildAndLog = { build: Build | undefined; log: Log; console: ConsoleLine[] };

export const noBuild: BuildAndLog = { build: undefined, log: emptyLog, console: emptyConsole };

// xcodebuild's step names, as the activity field reads them.
const stepVerbs: Record<string, string> = {
  SwiftCompile: "Compiling",
  CompileSwift: "Compiling",
  CompileSwiftSources: "Compiling",
  SwiftDriver: "Compiling",
  SwiftEmitModule: "Compiling",
  CompileC: "Compiling",
  CompileAssetCatalog: "Compiling assets of",
  CompileStoryboard: "Compiling storyboards of",
  Ld: "Linking",
  CodeSign: "Signing",
};

/** The last step in a batch of log text that names its target, "Compiling ChordKit", or null. */
export function buildStep(text: string): string | null {
  const lines = text.split("\n");
  for (let i = lines.length - 1; i >= 0; i--) {
    const step = /^(\S+) .*\(in target '([^']+)' from project /.exec(lines[i]);
    const verb = step && stepVerbs[step[1]];
    if (verb) return `${verb} ${step[2]}`;
  }
  return null;
}

const withStep = (build: Build, text: string): Build => {
  const step = buildStep(text);
  return step ? { ...build, step } : build;
};

// `startedAt` is when the build began compiling: the time Preparing and Waiting took does not count.
export function appliedEvent(state: BuildAndLog, event: BuildEvent_Deserialize, now: number): BuildAndLog {
  const build = state.build ?? { phase: "preparing", startedAt: now, issues: [], more: 0, endSeen: false };
  switch (event.kind) {
    case "phase":
      return { ...state, build: { ...build, phase: event.phase, startedAt: event.phase === "building" ? now : build.startedAt } };
    case "log":
      return { ...state, build: withStep(build, event.text), log: appendToLog(state.log, event.text) };
    case "console":
      return { ...state, console: appendToConsole(state.console, event.text, event.tone) };
    case "succeeded":
      return { ...state, build: { ...build, phase: "succeeded", endedAt: now } };
    case "exited":
      return { ...state, build: { ...build, phase: "exited", endedAt: now } };
    case "stopped":
      return { ...state, build: { ...build, phase: "stopped", endedAt: now } };
    case "failed":
      return { ...state, build: { ...build, phase: "failed", issues: event.issues, more: event.more, endedAt: now } };
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
      return { ...state, build: { ...state.build, side: withStep(side, event.text) }, log: appendToLog(state.log, event.text) };
    case "succeeded":
    case "stopped":
      return { ...state, build: { ...state.build, side: { ...side, phase: event.kind, endedAt: now } } };
    case "failed":
      return { ...state, build: { ...state.build, side: { ...side, phase: "failed", issues: event.issues, more: event.more, endedAt: now } } };
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

type Marker = { text: string; tone: "busy" | "done" | "stopped" | "failed" };

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

/** What the toolbar's activity field says: `lead` in `ink`, then `detail`, which carries its own lead-in ("· 10:32", "on iPhone 17 Pro"); the build beside a running app first, then the last build; `errors` counts a failed build's errors. */
export type Activity = { lead: string; detail: string | null; tone: "idle" | "busy" | "running" | "done" | "failed"; errors: number };

/** `scheme` is the scheme's name once its listing came; `time` reads an end's time, as a message's does. */
export function activityOf(build: Build | undefined, scheme: string | null, destination: string, time: (at: number) => string): Activity {
  const of = (verb: string) => (scheme ? `${verb} ${scheme}` : verb);
  const ended = (b: Build) => (b.endedAt === undefined ? null : `· ${time(b.endedAt)}`);
  const failed = (b: Build): Activity => ({ lead: "Build failed", detail: ended(b), tone: "failed", errors: b.issues.length + b.more });
  const building = (b: Build): Activity => ({ lead: of("Building"), detail: b.step ? `· ${b.step}` : null, tone: "busy", errors: 0 });
  if (!build) return { lead: "No build yet", detail: null, tone: "idle", errors: 0 };
  if (build.side && isStarting(build.side)) return building(build.side);
  if (build.side?.phase === "failed") return failed(build.side);
  switch (build.phase) {
    case "preparing":
    case "building":
      return building(build);
    case "waiting":
      return { lead: of("Building"), detail: "· Waiting for another build", tone: "busy", errors: 0 };
    case "installing":
      return { lead: of("Installing"), detail: `on ${destination}`, tone: "busy", errors: 0 };
    case "launching":
      return { lead: of("Launching"), detail: `on ${destination}`, tone: "busy", errors: 0 };
    case "running":
      return { lead: of("Running"), detail: `on ${destination}`, tone: "running", errors: 0 };
    case "exited":
      return { lead: scheme ? `${scheme} exited` : "App exited", detail: ended(build), tone: "idle", errors: 0 };
    case "succeeded":
      return { lead: "Build succeeded", detail: ended(build), tone: "done", errors: 0 };
    case "stopped":
      return { lead: "Build stopped", detail: ended(build), tone: "idle", errors: 0 };
    case "failed":
      return failed(build);
  }
}

/** "LoginView.swift:42", or the file's name alone without a line; null without a file. */
export function issuePlace(issue: BuildIssue_Deserialize): string | null {
  const name = issue.file && fileName(issue.file);
  if (!name) return null;
  return issue.line ? `${name}:${issue.line}` : name;
}

// An error's file as a path inside the worktree, or null for a file outside it.
export function pathInWorktree(worktree: string, file: string): string | null {
  return file.startsWith(`${worktree}/`) ? file.slice(worktree.length + 1) : null;
}
