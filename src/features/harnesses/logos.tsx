// Each harness's logo and title, by the name a pane saves.
import type { ReactNode } from "react";
import { ClaudeIcon, CodexIcon, PiIcon } from "../../ui/icons";

export const harnessMarks: Record<string, { Logo: () => ReactNode; title: string }> = {
  pi: { Logo: PiIcon, title: "pi" },
  codex: { Logo: CodexIcon, title: "Codex" },
  claude: { Logo: ClaudeIcon, title: "Claude Code" },
};
