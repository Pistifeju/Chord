import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import type { Checks, HarnessCheck } from "../../ipc/bindings";
import { CopyButton } from "../../ui/CopyButton";
import { BranchIcon, CheckIcon } from "../../ui/icons";
import { harnessMarks } from "./logos";
import { neededCommand, shownHarnesses, stateLine } from "./harnessCheck";
import { checkHarnesses, harnessChecks, pickHarness } from "./store";
import "./harnesses.css";

function Command({ text }: { text: string }) {
  const code = useRef<HTMLElement>(null);
  const [clipped, setClipped] = useState(false);
  useLayoutEffect(() => {
    const el = code.current;
    if (el) setClipped(el.scrollWidth > el.clientWidth);
  }, [text]);
  return (
    <span className="harness-command">
      <code ref={code} data-selectable data-clipped={clipped || undefined} title={text}>
        {text}
      </code>
      <CopyButton text={text} label="Copy Command" className="harness-copy" />
    </span>
  );
}

function Row({ logo, title, state, side, pick, current }: { logo: ReactNode; title: string; state: string; side: ReactNode; pick?: () => void; current?: boolean }) {
  const pickable = pick !== undefined;
  return (
    <div
      className="harness-row"
      data-pickable={pickable || undefined}
      role={pickable || current ? "radio" : undefined}
      aria-checked={pickable || current ? !!current : undefined}
      tabIndex={pickable ? 0 : undefined}
      title={pickable ? "Use for New Sessions" : undefined}
      onClick={pick}
      onKeyDown={(e) => {
        if (pick && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          pick();
        }
      }}
    >
      <span className="harness-logo">{logo}</span>
      <span className="harness-text">
        <span>{title}</span>
        <span key={state} className="harness-state">
          {state}
        </span>
      </span>
      <span className="harness-side">{side}</span>
    </div>
  );
}

function harnessSide(harness: HarnessCheck, chosen: boolean, drawCheck: boolean) {
  if (chosen) return <span key={harness.name} className="harness-check" data-draw={drawCheck || undefined} aria-hidden="true"><CheckIcon /></span>;
  const command = neededCommand(harness);
  return command && <Command key={command} text={command} />;
}

export function HarnessRows({ checks, rows }: { checks: Checks | null; rows?: HarnessCheck[] }) {
  const defaultHarness = checks?.defaultHarness;
  const previousDefault = useRef(defaultHarness);
  const drawCheck = previousDefault.current !== undefined && previousDefault.current !== defaultHarness;
  useEffect(() => {
    previousDefault.current = defaultHarness;
  }, [defaultHarness]);

  if (!checks) {
    return (
      <div className="harness-group" aria-busy="true">
        <Row logo={null} title="Agents" state="Checking" side={<span className="spinner" role="status" aria-label="Checking" />} />
      </div>
    );
  }
  return (
    <>
      <div className="harness-group" role="radiogroup" aria-label="Agent for new sessions">
        {(rows ?? shownHarnesses(checks)).map((harness) => {
          const chosen = harness.name === checks.defaultHarness;
          const Logo = harnessMarks[harness.name]?.Logo;
          return (
            <Row
              key={harness.name}
              logo={Logo && <Logo />}
              title={harness.title}
              state={stateLine(harness)}
              side={harnessSide(harness, chosen, drawCheck)}
              current={chosen}
              pick={!chosen && harness.takesPick ? () => pickHarness(harness.name) : undefined}
            />
          );
        })}
      </div>
      {!rows && checks.gitInstall && (
        <div className="harness-group">
          <Row logo={<BranchIcon />} title="Git" state="Not installed, needed for worktrees" side={<Command text={checks.gitInstall} />} />
        </div>
      )}
    </>
  );
}

export function CheckAgain() {
  const running = harnessChecks.usePart((s) => s.running && s.asked);
  return (
    <button type="button" className="button-secondary harness-recheck" aria-busy={running} onClick={() => void checkHarnesses(true)}>
      <span className="button-label">Check Again</span>
      {running && <span className="spinner" />}
    </button>
  );
}
