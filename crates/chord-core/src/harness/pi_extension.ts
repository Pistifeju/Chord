// The extension Chord loads into pi: the session commands RPC lacks, a pick of a session to resume and of a point in the session tree.
import { SessionManager, type ExtensionAPI } from "@earendil-works/pi-coding-agent";

type Content = string | { type: string; text?: string }[];

const textOf = (content: Content): string =>
  typeof content === "string"
    ? content
    : content
        .filter((block) => block.type === "text")
        .map((block) => block.text ?? "")
        .join("\n");

const firstLine = (text: string): string => (text.split("\n").find((line) => line.trim()) ?? "").trim().slice(0, 80);

// No-break spaces, so the indent of a branch survives in a one-line option.
const indent = "\u00a0\u00a0\u00a0\u00a0";

// Options are numbered, so two with the same text stay apart and the pick is found by its place.
const numbered = (labels: string[]): string[] => labels.map((label, i) => `${i + 1}. ${label}`);

export default function (pi: ExtensionAPI) {
  pi.registerCommand("chord-resume", {
    description: "Open another session of this worktree",
    handler: async (args, ctx) => {
      // Chord passes the session files other panes hold, so no two agents write one file.
      const held = new Set<string>(args.trim() ? JSON.parse(args) : []);
      const current = ctx.sessionManager.getSessionFile();
      const all = await SessionManager.list(ctx.cwd, ctx.sessionManager.getSessionDir());
      const sessions = all.filter((session) => session.path !== current && !held.has(session.path) && session.messageCount > 0);
      if (sessions.length === 0) throw new Error("This worktree has no other session to resume");
      const options = numbered(sessions.map((session) => `${firstLine(session.name ?? session.firstMessage)} · ${session.modified.toLocaleString()}`));
      const choice = await ctx.ui.select("Resume Session", options);
      const picked = choice === undefined ? undefined : sessions[options.indexOf(choice)];
      if (picked) await ctx.switchSession(picked.path);
    },
  });

  pi.registerCommand("chord-tree", {
    description: "Go back to an earlier point of this session",
    handler: async (_args, ctx) => {
      const rows: { id: string; user: boolean; text: string; label: string }[] = [];
      // A branch point indents what follows it, so a straight run of messages stays flat.
      const walk = (nodes: ReturnType<typeof ctx.sessionManager.getTree>, depth: number) => {
        for (const { entry, children } of nodes) {
          if (entry.type === "message" && (entry.message.role === "user" || entry.message.role === "assistant")) {
            const text = textOf(entry.message.content as Content);
            const who = entry.message.role === "user" ? "You" : "Agent";
            if (text.trim()) rows.push({ id: entry.id, user: entry.message.role === "user", text, label: `${indent.repeat(depth)}${who}: ${firstLine(text)}` });
          }
          walk(children, children.length > 1 ? depth + 1 : depth);
        }
      };
      walk(ctx.sessionManager.getTree(), 0);
      if (rows.length === 0) throw new Error("The session has no message to go back to");
      // The leaf is often an entry the list leaves out, so the last listed message of its branch is marked.
      const branch = new Set(ctx.sessionManager.getBranch().map((entry) => entry.id));
      const current = rows.findLast((row) => branch.has(row.id));
      if (current) current.label += " (current)";
      const options = numbered(rows.map((row) => row.label));
      const choice = await ctx.ui.select("Session Tree", options);
      const picked = choice === undefined ? undefined : rows[options.indexOf(choice)];
      if (!picked) return;
      const result = await ctx.navigateTree(picked.id, { summarize: false });
      // A user message is left out of the branch and goes back in the composer to edit and send again.
      if (!result.cancelled && picked.user) ctx.ui.setEditorText(picked.text);
    },
  });
}
