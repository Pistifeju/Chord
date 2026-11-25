import { commands, type Checks } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { shownHarnesses, withPick } from "./harnessCheck";

export const harnessChecks = externalStore<{ checks: Checks | null; running: boolean; asked: boolean }>({ checks: null, running: false, asked: false });

let pickDuringCheck: string | null = null;

export async function checkHarnesses(asked = false) {
  const current = harnessChecks.get();
  if (current.running) {
    if (asked && !current.asked) harnessChecks.set({ ...current, asked });
    return;
  }
  pickDuringCheck = null;
  harnessChecks.set({ ...current, running: true, asked });
  try {
    const checks = await commands.checkHarnesses();
    const picked = pickDuringCheck as string | null;
    const pick = picked && shownHarnesses(checks).find((harness) => harness.name === picked);
    harnessChecks.set({ checks: pick && pick.takesPick ? withPick(checks, pick.name) : checks, running: false, asked: false });
  } catch (error) {
    console.error(error);
    harnessChecks.set({ ...harnessChecks.get(), running: false, asked: false });
  }
}

export function pickHarness(name: string) {
  const current = harnessChecks.get();
  if (!current.checks) return;
  if (current.running) pickDuringCheck = name;
  harnessChecks.set({ ...current, checks: withPick(current.checks, name) });
  commands.changeState({ defaultHarness: name }).catch(console.error);
}

export function watchHarnessChecks() {
  void checkHarnesses();
  const again = () => void checkHarnesses();
  window.addEventListener("focus", again);
  return () => window.removeEventListener("focus", again);
}
