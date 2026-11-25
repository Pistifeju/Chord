import type { ConsoleTone } from "../../../ipc/bindings";
import { cappedLines } from "./log.ts";

export type ConsoleLine = { text: string; tone: ConsoleTone };

export const emptyConsole: ConsoleLine[] = [];

export function appendToConsole(lines: ConsoleLine[], text: string, tone: ConsoleTone): ConsoleLine[] {
  const added = text.replace(/\n$/, "").split("\n").map((line) => ({ text: line, tone }));
  return cappedLines(lines.concat(added));
}
