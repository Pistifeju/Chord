// The Settings page: one page of rows for the theme, the text size and every command's shortcut.
import { useState, type KeyboardEvent } from "react";
import type { Theme } from "../../ipc/bindings";
import { changeShortcut, commandTitles, resetShortcut, shortcutLabel, tooltip } from "../../app/commands";
import { recordedShortcut } from "../../app/shortcuts";
import { MinusIcon, PlusIcon } from "../../ui/icons";
import { appearance, changeTextSize, setTheme, shortcutChanges } from "./store";
import "./settings.css";

const themes: { value: Theme; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

function ShortcutRows() {
  const changes = shortcutChanges.use();
  const [recording, setRecording] = useState<string>();
  const [notice, setNotice] = useState("");

  const announce = (lost?: string) => setNotice(lost ? `${lost} lost its shortcut.` : "");

  const record = (id: string, e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") return setRecording(undefined);
    const shortcut = recordedShortcut(e.nativeEvent);
    if (!shortcut) return;
    announce(changeShortcut(id, shortcut));
    setRecording(undefined);
  };

  return (
    <>
      <div className="settings-section-header">
        <h2 className="settings-heading">Shortcuts</h2>
        <p className="settings-notice" role="status">
          {notice}
        </p>
      </div>
      {commandTitles.map(({ id, title }) => (
        <div key={id} className="settings-row">
          <span className="settings-label">{title}</span>
          <div className="settings-stepper">
            {id in changes && (
              <button type="button" className="button-secondary" aria-label={`Reset shortcut for ${title}`} onClick={() => announce(resetShortcut(id))}>
                Reset
              </button>
            )}
            <button
              type="button"
              className="button-secondary settings-key"
              aria-label={`Shortcut for ${title}`}
              data-recording={recording === id}
              data-empty={!shortcutLabel(id)}
              onClick={() => {
                setNotice("");
                setRecording(id);
              }}
              onKeyDown={recording === id ? (e) => record(id, e) : undefined}
              onBlur={() => setRecording(undefined)}
              ref={(button) => {
                if (recording === id) button?.focus();
              }}
            >
              {recording === id ? "Press keys" : (shortcutLabel(id) ?? "None")}
            </button>
          </div>
        </div>
      ))}
    </>
  );
}

export function SettingsPage() {
  const { theme, textPercent } = appearance.use();
  return (
    <section className="settings" aria-label="Settings">
      <header className="settings-header">
        <h1 className="settings-title">Settings</h1>
      </header>
      <div className="settings-row">
        <span className="settings-label">Theme</span>
        <div className="segmented" role="radiogroup" aria-label="Theme">
          {themes.map(({ value, label }) => (
            <button key={value} type="button" role="radio" aria-checked={theme === value} onClick={() => setTheme(value)}>
              {label}
            </button>
          ))}
        </div>
      </div>
      <div className="settings-row">
        <span className="settings-label">Text size</span>
        <div className="settings-stepper">
          <button type="button" className="button-secondary settings-step" aria-label="Decrease text size" title={tooltip("text.smaller")} onClick={() => changeTextSize("smaller")}>
            <MinusIcon />
          </button>
          <span className="settings-value">{textPercent}%</span>
          <button type="button" className="button-secondary settings-step" aria-label="Increase text size" title={tooltip("text.larger")} onClick={() => changeTextSize("larger")}>
            <PlusIcon />
          </button>
          <button type="button" className="button-secondary" title={tooltip("text.reset")} onClick={() => changeTextSize("reset")}>
            Reset
          </button>
        </div>
      </div>
      <ShortcutRows />
    </section>
  );
}
