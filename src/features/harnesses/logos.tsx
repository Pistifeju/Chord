import type { ReactNode } from "react";
import { ChatIcon, ClaudeIcon, CodexIcon, OpenCodeIcon, PiIcon } from "../../ui/icons";

export const harnessMarks: Record<string, { Logo: () => ReactNode; title: string }> = {
  pi: { Logo: PiIcon, title: "pi" },
  codex: { Logo: CodexIcon, title: "Codex" },
  claude: { Logo: ClaudeIcon, title: "Claude Code" },
  opencode: { Logo: OpenCodeIcon, title: "OpenCode" },
};

export const chatLogo = (harness: string | null): (() => ReactNode) => (harness && harnessMarks[harness]?.Logo) || ChatIcon;
