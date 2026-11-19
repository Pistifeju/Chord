// The first launch's three-step welcome: what Chord does, the agent check, the first project.
import { useEffect, useState } from "react";
import { tooltip } from "../../app/commands";
import { FolderIcon } from "../../ui/icons";
import { shortcutChanges } from "../settings/store";
import { StatusMark } from "../status/StatusMark";
import { statusLabel } from "../status/status";
import { pickProject } from "../worktrees/store";
import { CheckAgain, HarnessRows } from "../harnesses/HarnessRows";
import { agentHeading, rowsNote, shownHarnesses } from "../harnesses/harnessCheck";
import { harnessChecks, watchHarnessChecks } from "../harnesses/store";
import "./onboarding.css";

type Task = "running" | "waiting" | "done";

const TASKS = [
  ["Fix login redirect", "fix-login"],
  ["Add dark mode", "dark-mode"],
  ["Upgrade SDK", "sdk-17"],
] as const;

const BEATS: Task[][] = [
  ["running", "waiting", "done"],
  ["running", "running", "done"],
  ["waiting", "running", "running"],
  ["waiting", "done", "running"],
  ["running", "done", "waiting"],
  ["running", "running", "waiting"],
];
const BEAT_MS = 1800;

/** The open card follows the agent that waits, as the user would. */
function openCard(beat: number): number {
  for (let back = 0; back < BEATS.length; back++) {
    const waiting = BEATS[(beat - back + BEATS.length) % BEATS.length].indexOf("waiting");
    if (waiting >= 0) return waiting;
  }
  return 0;
}

function SidebarLoop() {
  const [beat, setBeat] = useState(0);
  useEffect(() => {
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const timer = setInterval(() => setBeat((b) => (b + 1) % BEATS.length), BEAT_MS);
    return () => clearInterval(timer);
  }, []);
  const open = openCard(beat);
  return (
    <div className="onboarding-sidebar" aria-hidden="true">
      <div className="onboarding-project">harbor</div>
      {TASKS.map(([title, branch], i) => {
        const kind = BEATS[beat][i];
        return (
          <div key={title} className="onboarding-card" data-open={i === open || undefined}>
            <div className="onboarding-card-head">
              <span className="onboarding-card-title">{title}</span>
              <span className="onboarding-branch">{branch}</span>
            </div>
            <div className="onboarding-session">
              <span className="onboarding-mark">
                <StatusMark kind={kind} />
              </span>
              Session 1
              <span key={kind} className="onboarding-status" data-status={kind}>
                {statusLabel({ kind })}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function Track({ step }: { step: number }) {
  return (
    <div className="onboarding-track" role="img" aria-label={`Step ${step + 1} of 3`}>
      {[0, 1, 2].map((n) => (
        <i key={n} data-at={n === step ? "now" : n < step ? "done" : undefined} />
      ))}
    </div>
  );
}

function Heading({ title, line, large }: { title: string; line: string; large?: boolean }) {
  return (
    <div className="onboarding-head">
      <h1 className="onboarding-title" data-large={large || undefined}>
        {title}
      </h1>
      <p className="onboarding-line">{line}</p>
    </div>
  );
}

function AddProject() {
  shortcutChanges.use();
  return (
    <button type="button" className="button-primary" title={tooltip("project.add")} autoFocus onClick={pickProject}>
      Add Project
    </button>
  );
}

function AgentStep({ back, next }: { back: () => void; next: () => void }) {
  const { checks } = harnessChecks.use();
  const { title, line } = agentHeading(checks);
  const note = checks ? rowsNote(checks) : "Checking";
  const ready = !checks || checks.defaultHarness !== null;
  return (
    <>
      <Heading title={title} line={line} />
      <HarnessRows checks={checks} />
      <div className="onboarding-rows-foot">
        <p className="onboarding-note">{note}</p>
        <CheckAgain />
      </div>
      <div className="onboarding-foot">
        <button type="button" className="onboarding-back" onClick={back}>
          Back
        </button>
        <button type="button" className={ready ? "button-primary" : "button-secondary"} autoFocus onClick={next}>
          {ready ? "Continue" : "Skip for Now"}
        </button>
      </div>
    </>
  );
}

function ProjectStep({ back }: { back: () => void }) {
  const { checks } = harnessChecks.use();
  const chosen = checks && shownHarnesses(checks).find((harness) => harness.name === checks.defaultHarness);
  return (
    <>
      <Heading title="Add your first project" line="Pick a git repository or any folder on your Mac." />
      <div className="onboarding-drop">
        <span className="onboarding-folder">
          <FolderIcon size={28} />
        </span>
        <AddProject />
        <p className="onboarding-note">or drop a folder anywhere on the window</p>
      </div>
      <div className="onboarding-foot">
        <button type="button" className="onboarding-back" onClick={back}>
          Back
        </button>
        <p className="onboarding-note">
          {chosen ? (
            <>
              New sessions run <b>{chosen.title}</b>
            </>
          ) : (
            "No agent yet. Chord asks at the first session."
          )}
        </p>
      </div>
    </>
  );
}

function Welcome() {
  const [{ step, moved }, setStep] = useState<{ step: number; moved?: "next" | "back" }>({ step: 0 });
  const go = (to: number) => setStep({ step: to, moved: to > step ? "next" : "back" });
  return (
    <div className="onboarding">
      <Track step={step} />
      {/* Keyed by the step, so each step change plays the slide; the first step shows still. */}
      <div key={step} className="onboarding-step" data-moved={moved}>
        {step === 0 && (
          <>
            <Heading large title="Welcome to Chord" line="Run several coding agents at once, each in its own worktree. Chord shows you which one needs you next." />
            <SidebarLoop />
            <div className="onboarding-foot" data-center>
              <button type="button" className="button-primary" autoFocus onClick={() => go(1)}>
                Get Started
              </button>
            </div>
          </>
        )}
        {step === 1 && <AgentStep back={() => go(0)} next={() => go(2)} />}
        {step === 2 && <ProjectStep back={() => go(1)} />}
      </div>
    </div>
  );
}

export function Onboarding() {
  useEffect(watchHarnessChecks, []);
  return <Welcome />;
}
