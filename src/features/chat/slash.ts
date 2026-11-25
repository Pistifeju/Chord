import type { SlashCommand } from "../../ipc/bindings";

export const slashQuery = (draft: string): string | null => /^\/(\S*)$/.exec(draft)?.[1] ?? null;

function matchingCommands(commands: SlashCommand[], query: string): SlashCommand[] {
  const needle = query.toLowerCase();
  const at = (command: SlashCommand) => (command.name.toLowerCase() === needle ? -1 : command.name.toLowerCase().indexOf(needle));
  return commands
    .filter((command) => command.name.toLowerCase().includes(needle))
    .map((command, order) => ({ command, order }))
    .sort((a, b) => at(a.command) - at(b.command) || a.order - b.order)
    .map(({ command }) => command);
}

function withoutSkillAliases(commands: SlashCommand[]): SlashCommand[] {
  const skills = new Set(commands.filter((command) => command.kind === "skill").map((command) => command.name));
  return commands.filter((command) => command.kind !== "extension" || !skills.has(`skill:${command.name}`));
}

export function slashMenu(commands: SlashCommand[], draft: string, closedOn: string | undefined): SlashCommand[] {
  const query = slashQuery(draft);
  return query === null || closedOn === draft ? [] : matchingCommands(withoutSkillAliases(commands), query);
}

export const commandTitle = (name: string): string =>
  name
    .replace(/^skill:/, "")
    .split(/[-_:]+/)
    .filter(Boolean)
    .map((word) => word[0].toUpperCase() + word.slice(1))
    .join(" ");

export function markedTitle(title: string, query: string): { before: string; typed: string; after: string } {
  const needle = query.toLowerCase().replace(/[-_:\s]/g, "");
  const letters: number[] = [];
  [...title].forEach((char, i) => char !== " " && letters.push(i));
  const squeezed = letters.map((i) => title[i].toLowerCase()).join("");
  for (let at = squeezed.indexOf(needle); needle && at >= 0; at = squeezed.indexOf(needle, at + 1)) {
    if (at === 0 || title[letters[at] - 1] === " ") {
      const start = letters[at];
      const end = letters[at + needle.length - 1] + 1;
      return { before: title.slice(0, start), typed: title.slice(start, end), after: title.slice(end) };
    }
  }
  return { before: title, typed: "", after: "" };
}

export function commandSource(command: SlashCommand): string {
  if (command.origin) return command.origin;
  if (command.kind === "session") return "Session";
  return command.kind === "extension" ? "Extension" : "Local file";
}

export const completion = (command: SlashCommand): string => `/${command.name} `;
