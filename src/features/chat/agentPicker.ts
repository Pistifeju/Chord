import type { Checks, ModelOption } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore.ts";
import { shownHarnesses } from "../harnesses/harnessCheck.ts";
import type { ModelChoice } from "./transcript.ts";

export const agentPickerPane = externalStore<string | null>(null);

export const openAgentPicker = (pane: string) => agentPickerPane.set(pane);

export const closeAgentPicker = () => agentPickerPane.set(null);

export function pickerAgents(checks: Checks | null, own: string): string[] {
  const ready = checks ? shownHarnesses(checks).filter((h) => h.installState.kind === "ready" || h.installState.kind === "outdated").map((h) => h.name) : [];
  return ready.includes(own) ? ready : [own, ...ready];
}

export function shownLevel(model: ModelOption, running: ModelChoice | null, picked: string | null): string | null {
  if (model.levels.length === 0) return null;
  if (picked && model.levels.includes(picked)) return picked;
  if (running?.thinking && model.levels.includes(running.thinking)) return running.thinking;
  return model.default_level ?? null;
}

export function levelSlots(models: ModelOption[]): string[] {
  const slots: string[] = [];
  for (const model of models) {
    model.levels.forEach((level, at) => {
      if (slots.includes(level)) return;
      const after = at === 0 ? -1 : slots.indexOf(model.levels[at - 1]);
      slots.splice(after + 1, 0, level);
    });
  }
  return slots;
}
