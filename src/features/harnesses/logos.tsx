// Each harness's logo and title, by the name a pane saves.
import type { ReactNode } from "react";
import { CodexIcon, PiIcon } from "../../ui/icons";

export const harnessMarks: Record<string, { Logo: () => ReactNode; title: string }> = {
  pi: { Logo: PiIcon, title: "pi" },
  codex: { Logo: CodexIcon, title: "Codex" },
};
