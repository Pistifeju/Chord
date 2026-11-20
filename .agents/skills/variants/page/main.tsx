// The variants page: four versions of one request side by side, in light or dark.
import { StrictMode, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/inter";
import "../../../src/ui/tokens.css";
import "../../../src/ui/controls.css";
import "./variants.css";
import { Segmented } from "../../../src/ui/Segmented";

const request = "The request";

const versions: { name: string; look: string; act: string; view: ReactNode }[] = [
  { name: "Name", look: "Look move", act: "Act move", view: <Version1 /> },
  { name: "Name", look: "Look move", act: "Act move", view: <Version2 /> },
  { name: "Name", look: "Look move", act: "Act move", view: <Version3 /> },
  { name: "Name", look: "Look move", act: "Act move", view: <Version4 /> },
];

function Version1() {
  return <button className="button-primary">Version 1</button>;
}

function Version2() {
  return <button className="button-secondary">Version 2</button>;
}

function Version3() {
  return <button className="button-secondary">Version 3</button>;
}

function Version4() {
  return <button className="button-secondary">Version 4</button>;
}

const appearances = [
  { value: false, label: "Light" },
  { value: true, label: "Dark" },
];

function Page() {
  const [dark, setDark] = useState(() => matchMedia("(prefers-color-scheme: dark)").matches);
  useEffect(() => {
    if (dark) document.documentElement.dataset.theme = "dark";
    else delete document.documentElement.dataset.theme;
  }, [dark]);
  return (
    <main className="variants">
      <header className="variants-header">
        <h1>{request}</h1>
        <Segmented label="Appearance" options={appearances} value={dark} onChange={setDark} />
      </header>
      {versions.map((version, index) => (
        <section key={index} className="variants-version">
          <h2>
            {index + 1} · {version.name}
          </h2>
          <p>
            {version.look} · {version.act}
          </p>
          <div className="variants-stage">{version.view}</div>
        </section>
      ))}
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Page />
  </StrictMode>,
);
