import { connect } from "node:net";
import { SessionManager, type ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";

type Content = string | { type: string; text?: string }[];

const textOf = (content: Content): string =>
  typeof content === "string"
    ? content
    : content
        .filter((block) => block.type === "text")
        .map((block) => block.text ?? "")
        .join("\n");

const firstLine = (text: string): string => (text.split("\n").find((line) => line.trim()) ?? "").trim().slice(0, 80);

const indent = "\u00a0\u00a0\u00a0\u00a0";

const numbered = (labels: string[]): string[] => labels.map((label, i) => `${i + 1}. ${label}`);

const USAGE_STATUS = "chord-usage";
const usageIds = ["sub-hourly", "sub-weekly"];
const unitMs: Record<string, number> = { d: 86400000, h: 3600000, m: 60000 };

type UsageWindow = { label: string; resetsAt: number | null };

function usageWindow(update: { text?: string; suffix?: string }): UsageWindow {
  const text = String(update.text ?? "");
  const left = text.match(/\s(now|\d+[dhm](?:\d+[hm])*)$/);
  const name = (left ? text.slice(0, left.index) : text).trim();
  if (!left) return { label: `${name} ${update.suffix ?? ""}`.trim(), resetsAt: null };
  if (left[1] === "now") return { label: `${name} 0%`, resetsAt: null };
  const label = `${name} ${update.suffix ?? ""}`.trim();
  const ms = [...left[1].matchAll(/(\d+)([dhm])/g)].reduce((total, [, n, unit]) => total + Number(n) * unitMs[unit], 0);
  return { label, resetsAt: Date.now() + ms };
}

type Reply = { text: string; failed: boolean };

function callChord(socket: string, tool: string, worktree: string, signal: AbortSignal | undefined): Promise<Reply> {
  return new Promise((resolve, reject) => {
    const connection = connect(socket, () => connection.write(`${JSON.stringify({ tool, worktree })}\n`));
    let answer = "";
    connection.setEncoding("utf8");
    connection.on("data", (chunk: string) => (answer += chunk));
    connection.on("end", () => {
      try {
        resolve(JSON.parse(answer) as Reply);
      } catch {
        reject(new Error("Chord closed the call without an answer"));
      }
    });
    connection.on("error", (error) => reject(new Error(`Chord cannot take the call: ${error.message}`)));
    signal?.addEventListener("abort", () => {
      connection.destroy();
      reject(new Error("Aborted. The build goes on in Chord"));
    });
  });
}

type ToolTexts = {
  instructions: string[];
  tools: { name: string; label: string; description: string; promptSnippet: string }[];
};

function registerBuildTools(pi: ExtensionAPI, socket: string, worktree: string, texts: ToolTexts) {
  texts.tools.forEach((tool, index) => {
    pi.registerTool({
      ...tool,
      promptGuidelines: index === 0 ? texts.instructions : [],
      parameters: Type.Object({}),
      async execute(_id, _params, signal) {
        const reply = await callChord(socket, tool.name, worktree, signal);
        return { content: [{ type: "text", text: reply.text }], details: undefined, isError: reply.failed };
      },
    });
  });
}

export default function (pi: ExtensionAPI) {
  const socket = process.env.CHORD_TOOLS;
  const worktree = process.env.CHORD_WORKTREE;
  const texts = process.env.CHORD_TOOL_TEXTS;
  if (socket && worktree && texts) registerBuildTools(pi, socket, worktree, JSON.parse(texts) as ToolTexts);

  const usage: Record<string, UsageWindow> = {};
  let ui: { setStatus(key: string, text: string | undefined): void } | undefined;
  let queued = false;
  const showUsage = () => {
    if (queued) return;
    queued = true;
    queueMicrotask(() => {
      queued = false;
      const windows = usageIds.flatMap((id) => (usage[id] ? [usage[id]] : []));
      if (windows.length > 0) ui?.setStatus(USAGE_STATUS, JSON.stringify(windows));
    });
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

  pi.registerCommand("chord-reload", {
    description: "Reload extensions, skills, prompts and context files",
    handler: async (_args, ctx) => {
      await ctx.reload();
    },
  });

  pi.registerCommand("chord-tree", {
    description: "Go back to an earlier point of this session",
    handler: async (_args, ctx) => {
      const rows: { id: string; user: boolean; text: string; label: string }[] = [];
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
      const branch = new Set(ctx.sessionManager.getBranch().map((entry) => entry.id));
      const current = rows.findLast((row) => branch.has(row.id));
      if (current) current.label += " (current)";
      const options = numbered(rows.map((row) => row.label));
      const choice = await ctx.ui.select("Session Tree", options);
      const picked = choice === undefined ? undefined : rows[options.indexOf(choice)];
      if (!picked) return;
      const result = await ctx.navigateTree(picked.id, { summarize: false });
      if (!result.cancelled && picked.user) ctx.ui.setEditorText(picked.text);
    },
  });
}
