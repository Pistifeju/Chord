// Rules for the console's lines: the running app's output as whole lines, each with its tone, capped like the build log.
import type { ConsoleTone } from "../../../ipc/bindings";
import { cappedLines } from "./log.ts";

export type ConsoleLine = { text: string; tone: ConsoleTone };

export const emptyConsole: ConsoleLine[] = [];

// `text` holds whole lines, each ending with its break.
export function appendToConsole(lines: ConsoleLine[], text: string, tone: ConsoleTone): ConsoleLine[] {
  const added = text.replace(/\n$/, "").split("\n").map((line) => ({ text: line, tone }));
  return cappedLines(lines.concat(added));
}
