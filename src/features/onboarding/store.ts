// The harness checks the window holds: the last result, whether a check runs, and the user's pick of the default harness, saved through Rust.
import { commands, type Checks } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { shownHarnesses, withPick } from "./harnessCheck";

export const harnessChecks = externalStore<{ checks: Checks | null; running: boolean }>({ checks: null, running: false });

// A pick made while a check runs wins over the default that check read before it.
let pickDuringCheck: string | null = null;

/** Checks every harness and git; the rows keep the last result while it runs. A call while one runs does nothing. */
export async function checkHarnesses() {
  if (harnessChecks.get().running) return;
  pickDuringCheck = null;
  harnessChecks.set({ ...harnessChecks.get(), running: true });
  try {
    const checks = await commands.checkHarnesses();
    // Set by `pickHarness` during the await, which the narrowing above cannot see.
    const picked = pickDuringCheck as string | null;
    const pick = picked && shownHarnesses(checks).find((harness) => harness.name === picked);
    harnessChecks.set({ checks: pick && pick.takesPick ? withPick(checks, pick.name) : checks, running: false });
  } catch (error) {
    console.error(error);
    harnessChecks.set({ ...harnessChecks.get(), running: false });
  }
}

export function pickHarness(name: string) {
  const { checks, running } = harnessChecks.get();
  if (!checks) return;
  if (running) pickDuringCheck = name;
  harnessChecks.set({ checks: withPick(checks, name), running });
  commands.changeState({ defaultHarness: name }).catch(console.error);
}

/** Checks now and each time the window comes forward, since the user installs or signs in outside Chord. */
export function watchHarnessChecks() {
  void checkHarnesses();
  const again = () => void checkHarnesses();
  window.addEventListener("focus", again);
  return () => window.removeEventListener("focus", again);
}
