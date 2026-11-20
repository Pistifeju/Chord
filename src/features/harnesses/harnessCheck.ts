// Rules for the harness rows: which harnesses show, each row's state line and the command it needs, the agent screen's title, and the rows a chat pane shows in place of Start.
import type { Checks, HarnessCheck } from "../../ipc/bindings";

export const shownHarnesses = (checks: Checks): HarnessCheck[] => checks.harnesses.filter((harness) => harness.starts);

export function stateLine({ installState: state }: HarnessCheck): string {
  switch (state.kind) {
    case "ready":
      return `Version ${state.version}`;
    case "missing":
      return "Not installed";
    case "signed_out":
      return "Installed, not signed in";
    case "outdated":
      return `Version ${state.version}, tested from ${state.oldest}`;
  }
}

/** The command the user runs before Chord can start the harness; none when it is ready. */
export function neededCommand({ installState: state, install, signIn, upgrade }: HarnessCheck): string | null {
  switch (state.kind) {
    case "ready":
      return null;
    case "missing":
      return install;
    case "signed_out":
      return signIn ?? null;
    case "outdated":
      return upgrade;
  }
}

export const withPick = (checks: Checks, name: string): Checks => ({ ...checks, defaultHarness: name });

/** The agent screen's title and line: what the check found. */
export function agentHeading(checks: Checks | null): { title: string; line: string } {
  const own = "Chord runs the agent you already use, under your own login.";
  if (!checks) return { title: "Your agent", line: own };
  const chosen = shownHarnesses(checks).find((harness) => harness.name === checks.defaultHarness);
  if (!chosen) return { title: "Install an agent", line: `${own} It never installs or signs in for you.` };
  if (shownHarnesses(checks).filter((harness) => harness.takesPick).length > 1) return { title: "Pick your agent", line: "Chord runs the agents you already use, under your own login. Click one to make it the default." };
  return { title: `${chosen.title} is ready`, line: `New sessions run ${chosen.title}, under your own login.` };
}

/** The line under the rows: how to get past a missing step, or what the pick does. */
export function rowsNote(checks: Checks): string | null {
  if (!checks.defaultHarness || checks.gitInstall) return "Run the command, then switch back. Chord checks again.";
  return shownHarnesses(checks).filter((harness) => harness.takesPick).length > 1 ? "New sessions use the checked agent." : null;
}

/** A pane without a harness shows every row while no default exists to take. */
export function blockingRows(checks: Checks | null, harness: string | null): HarnessCheck[] {
  if (!checks) return [];
  if (harness === null) return checks.defaultHarness ? [] : shownHarnesses(checks);
  return shownHarnesses(checks).filter((check) => check.name === harness && !check.takesPick);
}
