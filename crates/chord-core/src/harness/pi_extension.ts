// The extension Chord loads into pi: the session commands RPC lacks, a pick of a session to resume and of a point in the session tree, and the subscription usage windows.
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

const USAGE_STATUS = "chord-usage";
const usageIds = ["sub-hourly", "sub-weekly"];
const unitMs: Record<string, number> = { d: 86400000, h: 3600000, m: 60000 };

type UsageWindow = { label: string; resetsAt: number | null };

// A reset left as a point in time stays right between updates; a window already reset is stale and left out.
function usageWindow(update: { text?: string; suffix?: string }): UsageWindow | undefined {
  const text = String(update.text ?? "");
  const left = text.match(/(?:^|\s)(now|\d+[dhm](?:\d+[hm])*)$/);
  const label = `${(left ? text.slice(0, left.index) : text).trim()} ${update.suffix ?? ""}`.trim();
  if (!left) return { label, resetsAt: null };
  if (left[1] === "now") return undefined;
  const ms = [...left[1].matchAll(/(\d+)([dhm])/g)].reduce((total, [, n, unit]) => total + Number(n) * unitMs[unit], 0);
  return { label, resetsAt: Date.now() + ms };
}

export default function (pi: ExtensionAPI) {
  // RPC drops the usage bar's TUI component, so the windows travel as a status text the adapter reads.
  const usage: Record<string, UsageWindow> = {};
  let ui: { setStatus(key: string, text: string | undefined): void } | undefined;
  const showUsage = () => {
    const windows = usageIds.flatMap((id) => (usage[id] ? [usage[id]] : []));
    ui?.setStatus(USAGE_STATUS, windows.length > 0 ? JSON.stringify(windows) : undefined);
  };
  pi.events.on("powerbar:update", (update: any) => {
    if (!usageIds.includes(update?.id)) return;
    const window = update.text || update.bar !== undefined ? usageWindow(update) : undefined;
    if (window) usage[update.id] = window;
    else delete usage[update.id];
    showUsage();
  });
  pi.on("session_start", (_event, ctx) => {
    ui = ctx.hasUI ? ctx.ui : undefined;
    showUsage();
  });

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
