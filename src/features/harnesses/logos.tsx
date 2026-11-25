// Each harness's logo and title, by the name a pane saves, and the logo a chat shows for its pane's harness.
import type { ReactNode } from "react";
import { ChatIcon, ClaudeIcon, CodexIcon, OpenCodeIcon, PiIcon } from "../../ui/icons";

export const harnessMarks: Record<string, { Logo: () => ReactNode; title: string }> = {
  pi: { Logo: PiIcon, title: "pi" },
  codex: { Logo: CodexIcon, title: "Codex" },
  claude: { Logo: ClaudeIcon, title: "Claude Code" },
  opencode: { Logo: OpenCodeIcon, title: "OpenCode" },
};

/** The logo of a chat's harness; the chat icon for a pane that never started, and for a harness without a logo. */
export const chatLogo = (harness: string | null): (() => ReactNode) => (harness && harnessMarks[harness]?.Logo) || ChatIcon;
