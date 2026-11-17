// Entry: loads the font and the tokens, mounts App.
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/inter";
import "./ui/tokens.css";
import "./ui/controls.css";
import App from "./app/App";

// The webview's own menu offers Reload, which drops all UI state; keep it only where it carries Copy/Paste.
document.addEventListener("contextmenu", (event) => {
  const target = event.target as HTMLElement;
  if (target.closest("input, textarea, [contenteditable]") || !document.getSelection()?.isCollapsed) return;
  event.preventDefault();
});

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
