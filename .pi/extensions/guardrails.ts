// Pi hooks that enforce three AGENTS.md rules: generated files stay generated, git history and other sessions' work stay safe, and a code edit ends with npm run check.
import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";

const GENERATED: Record<string, string> = {
  "src/ipc/bindings.ts": "Run `npm run bindings` to regenerate it.",
  "Cargo.lock": "Let cargo write it.",
};

const HISTORY = /\bgit\b[^|;&]*\s(commit|push)\b/;
// shortcut: a regex sees one spelling, `sh -c` or a script file passes. Move to a sandbox once an agent gets past it.
const DESTRUCTIVE =
  /\bgit\b[^|;&]*\s(reset\s+--hard|clean\b|stash\b(?!\s+(list|show))|restore\s+(?!--staged)|checkout\s+(--\s|\.(\s|$)))/;

export default function (pi: ExtensionAPI) {
  let userAskedForGit = false;
  let editedSinceCheck = false;
  let reminded = false;

  const confirmOrBlock = async (ctx: ExtensionContext, title: string, command: string, reason: string) => {
    if (ctx.hasUI && (await ctx.ui.confirm(title, command))) return undefined;
    return { block: true, reason };
  };

  pi.on("input", (event) => {
    if (event.source === "extension") return;
    userAskedForGit = /commit|push/i.test(event.text);
    reminded = false;
  });

  pi.on("tool_call", async (event, ctx) => {
    if (event.toolName === "edit" || event.toolName === "write") {
      const path = String(event.input.path ?? "");
      for (const [file, fix] of Object.entries(GENERATED)) {
        if (path === file || path.endsWith(`/${file}`)) {
          return { block: true, reason: `${file} is generated. ${fix}` };
        }
      }
      if (!path.endsWith(".md")) editedSinceCheck = true;
      return undefined;
    }

    if (event.toolName !== "bash") return undefined;
    const command = String(event.input.command ?? "");

    if (DESTRUCTIVE.test(command)) {
      return confirmOrBlock(
        ctx,
        "Run a git command that discards uncommitted work?",
        command,
        "This command discards uncommitted work, and other sessions share this tree. Ask the user to run it.",
      );
    }
    if (HISTORY.test(command) && !userAskedForGit) {
      return confirmOrBlock(
        ctx,
        "Let the agent commit or push?",
        command,
        "The user did not ask for a commit or push. Report the changes and let the user decide.",
      );
    }
    if (/\bnpm run check\b/.test(command)) editedSinceCheck = false;
    return undefined;
  });

  pi.on("agent_before_settle", (event) => {
    if (!editedSinceCheck || reminded || event.outcome !== "completed") return undefined;
    // One reminder per user message, so a check the agent cannot fix does not loop.
    reminded = true;
    return {
      entries: [
        ...event.entries,
        {
          type: "custom_message" as const,
          customType: "guardrails-check",
          content:
            "You edited code after the last `npm run check`. Run `npm run fix`, then `npm run check`, and report the result.",
          display: true,
        },
      ],
      continue: true,
    };
  });
}
