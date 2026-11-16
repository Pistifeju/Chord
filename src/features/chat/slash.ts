// Rules for the slash menu: when a draft opens it, which commands it lists, how a command shows its title and source, and what a pick writes.
import type { SlashCommand } from "../../ipc/bindings";

/** What follows the slash while the draft is only a command name; null once it holds a space or a line break or does not start with a slash. */
export const slashQuery = (draft: string): string | null => /^\/(\S*)$/.exec(draft)?.[1] ?? null;

/** The commands whose name holds the query, ignoring case: the exact name first, then names that start with it, the rest by where it sits. */
function matchingCommands(commands: SlashCommand[], query: string): SlashCommand[] {
  const needle = query.toLowerCase();
  const at = (command: SlashCommand) => (command.name.toLowerCase() === needle ? -1 : command.name.toLowerCase().indexOf(needle));
  return commands
    .filter((command) => command.name.toLowerCase().includes(needle))
    .map((command, order) => ({ command, order }))
    .sort((a, b) => at(a.command) - at(b.command) || a.order - b.order)
    .map(({ command }) => command);
}

/** The commands without an extension command named like a skill, which shares the skill's title and runs the same skill. */
function withoutSkillAliases(commands: SlashCommand[]): SlashCommand[] {
  const skills = new Set(commands.filter((command) => command.kind === "skill").map((command) => command.name));
  return commands.filter((command) => command.kind !== "extension" || !skills.has(`skill:${command.name}`));
}

/** The menu's rows for a draft; empty while the menu is closed. */
export function slashMenu(commands: SlashCommand[], draft: string, closedOn: string | undefined): SlashCommand[] {
  const query = slashQuery(draft);
  return query === null || closedOn === draft ? [] : matchingCommands(withoutSkillAliases(commands), query);
}

/** A command's display title: the name without a `skill:` prefix, split on `-`, `_` and `:`, each word capitalized. */
export const commandTitle = (name: string): string =>
  name
    .replace(/^skill:/, "")
    .split(/[-_:]+/)
    .filter(Boolean)
    .map((word) => word[0].toUpperCase() + word.slice(1))
    .join(" ");

/** The title split around the typed part, which must match from the start of a word ignoring case and separators; nothing is marked when it does not. */
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

/** The words at the row's right edge saying where the command comes from. */
export function commandSource(command: SlashCommand): string {
  if (command.origin) return command.origin;
  if (command.kind === "session") return "Session";
  return command.kind === "extension" ? "Extension" : "Local file";
}

/** What a pick writes into the composer: the name, then a space for its arguments. */
export const completion = (command: SlashCommand): string => `/${command.name} `;
