// The agent picker: the pane that shows it, the agents it offers, the thinking bar's segments, and the level the bar marks for a model.
import type { Checks, ModelOption } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore.ts";
import { shownHarnesses } from "../harnesses/harnessCheck.ts";
import type { ModelChoice } from "./transcript.ts";

/** The pane whose agent picker shows; one at a time. */
export const agentPickerPane = externalStore<string | null>(null);

export const openAgentPicker = (pane: string) => agentPickerPane.set(pane);

export const closeAgentPicker = () => agentPickerPane.set(null);

/** The agents the picker offers, in the checks' order: each one Chord runs that is installed and signed in, and the pane's own whatever its check says. */
export function pickerAgents(checks: Checks | null, own: string): string[] {
  const ready = checks ? shownHarnesses(checks).filter((h) => h.installState.kind === "ready" || h.installState.kind === "outdated").map((h) => h.name) : [];
  return ready.includes(own) ? ready : [own, ...ready];
}

/** The level the bar marks for a model: one the user set for it in the bar, else the running model's level when the model has it, else the model's default. Null leaves the level to the harness. */
export function shownLevel(model: ModelOption, running: ModelChoice | null, chosen: Record<string, string>): string | null {
  if (model.levels.length === 0) return null;
  const set = chosen[model.id];
  if (set) return set;
  if (running?.thinking && model.levels.includes(running.thinking)) return running.thinking;
  return model.default_level ?? null;
}

/** The thinking bar's segments for an agent: every level any of its models has, in the models' own order, so the bar keeps its layout as the highlight moves between models. */
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
