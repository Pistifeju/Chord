import type { BuildEvent_Deserialize, BuildIssue_Deserialize } from "../../../ipc/bindings";
import type { BottomViewSlot } from "../../bottompanel/BottomPanel";
import type { BottomView } from "../../../ipc/bindings";
import { appendToConsole, emptyConsole, type ConsoleLine } from "./consoleLog.ts";
import { appendToLog, emptyLog, type Log } from "./log.ts";
import { fileName } from "../../files/paths.ts";

export type Build = {
  phase: "preparing" | "waiting" | "building" | "installing" | "launching" | "running" | "exited" | "succeeded" | "failed" | "stopped";
  startedAt: number;
  issues: BuildIssue_Deserialize[];
  more: number;
  endSeen: boolean;
  endedAt?: number;
  step?: string;
  side?: Build;
};

export type BuildAndLog = { build: Build | undefined; log: Log; console: ConsoleLine[] };

export const noBuild: BuildAndLog = { build: undefined, log: emptyLog, console: emptyConsole };

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

export const isStarting = (build: Build | undefined): boolean =>
  build?.phase === "preparing" || build?.phase === "waiting" || build?.phase === "building" || build?.phase === "installing" || build?.phase === "launching";

export const isRunning = (build: Build | undefined): boolean => build?.phase === "running";

export const viewToShow = (build: Build | undefined): BottomViewSlot["id"] => (build?.phase === "running" || build?.phase === "exited" ? "console" : "build");

export const viewAfterEvent = (view: BottomView, event: BuildEvent_Deserialize): BottomView =>
  view === "build" && event.kind === "phase" && event.phase === "running" ? "console" : view;

type Marker = { text: string; tone: "busy" | "done" | "stopped" | "failed" };

export const seenEnd = (build: Build): Build =>
  ["succeeded", "installing", "launching", "stopped"].includes(build.phase) && !build.endSeen ? { ...build, endSeen: true } : build;

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

export type Activity = { lead: string; detail: string | null; tone: "idle" | "busy" | "running" | "done" | "failed"; errors: number };

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

export function issuePlace(issue: BuildIssue_Deserialize): string | null {
  const name = issue.file && fileName(issue.file);
  if (!name) return null;
  return issue.line ? `${name}:${issue.line}` : name;
}

export function pathInWorktree(worktree: string, file: string): string | null {
  return file.startsWith(`${worktree}/`) ? file.slice(worktree.length + 1) : null;
}

export function failedBuild(build: Build | undefined): Build | null {
  if (!build || (build.side && isStarting(build.side))) return null;
  if (build.side?.phase === "failed") return build.side;
  return build.phase === "failed" ? build : null;
}

export function issueGroups(issues: BuildIssue_Deserialize[]): { file: string | null; issues: BuildIssue_Deserialize[] }[] {
  const groups = new Map<string | null, BuildIssue_Deserialize[]>();
  for (const issue of issues) {
    const file = issue.file ?? null;
    groups.set(file, [...(groups.get(file) ?? []), issue]);
  }
  return [...groups].map(([file, grouped]) => ({ file, issues: grouped }));
}
