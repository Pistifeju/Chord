// Switches the session to Sonnet when /skill:implement-feature starts, since the main agent only runs subagents.
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
  pi.on("input", async (event, ctx) => {
    if (!event.text.startsWith("/skill:implement-feature")) return { action: "continue" };
    const model = ctx.modelRegistry.find("anthropic", "claude-sonnet-5-5");
    if (!model) {
      ctx.ui.notify("implement-feature: claude-sonnet-5-5 not found, keeping the current model", "warning");
      return { action: "continue" };
    }
    if (ctx.model?.id !== model.id && !(await pi.setModel(model))) {
      ctx.ui.notify("implement-feature: no API key for claude-sonnet-5-5", "warning");
    }
    pi.setThinkingLevel("medium");
    return { action: "continue" };
  });
}
