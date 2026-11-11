// Entry: loads the font and the tokens, mounts App.
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./ui/tokens.css";
import App from "./app/App";

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
