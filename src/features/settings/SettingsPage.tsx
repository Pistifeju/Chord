// The Settings panel over the blurred window: a section list on the left, the chosen section's grouped rows on the right.
import { useState, type KeyboardEvent, type ReactNode } from "react";
import type { Theme } from "../../ipc/bindings";
import { changeShortcut, commandTitles, resetShortcut, shortcutLabel, tooltip } from "../../app/commands";
import { sheet } from "../../app/sheet";
import { recordedShortcut } from "../../app/shortcuts";
import { AppearanceIcon, CloseIcon, KeyboardIcon, MinusIcon, PlusIcon, SlidersIcon } from "../../ui/icons";
import { appearance, changeTerminalFontSize, changeTextSize, closeSettings, keepAwake, setKeepAwake, setTheme, shortcutChanges } from "./store";
import "./settings.css";

const themes: { value: Theme; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

const sections = [
  { id: "general", title: "General", icon: <SlidersIcon /> },
  { id: "appearance", title: "Appearance", icon: <AppearanceIcon /> },
  { id: "shortcuts", title: "Shortcuts", icon: <KeyboardIcon /> },
] as const;

type SectionId = (typeof sections)[number]["id"];

function Row({ title, description, children }: { title: string; description?: ReactNode; children: ReactNode }) {
  return (
    <div className="settings-row">
      <div className="settings-row-text">
        <span className="settings-label">{title}</span>
        {description}
      </div>
      {children}
    </div>
  );
}

function Group({ heading, notice, children }: { heading: string; notice?: ReactNode; children: ReactNode }) {
  return (
    <section className="settings-section" aria-label={heading}>
      <div className="settings-section-header">
        <h3 className="settings-heading">{heading}</h3>
        {notice}
      </div>
      <div className="settings-group">{children}</div>
    </section>
  );
}

function General({ buildDataSize }: { buildDataSize: ReactNode }) {
  const enabled = keepAwake.use();
  const [error, setError] = useState("");
  return (
    <>
      <Group heading="Agents">
        <Row
          title="Keep the Mac awake"
          description={
            <>
              <p className="settings-description">The Mac does not sleep while an agent runs.</p>
              <p className="settings-description" data-error role="status">
                {error}
              </p>
            </>
          }
        >
          <div className="segmented" role="radiogroup" aria-label="Keep the Mac awake while an agent runs">
            {[true, false].map((value) => (
              <button
                key={String(value)}
                type="button"
                role="radio"
                aria-checked={enabled === value}
                onClick={() => setKeepAwake(value).then((failure) => setError(failure ?? ""))}
              >
                {value ? "On" : "Off"}
              </button>
            ))}
          </div>
        </Row>
      </Group>
      {buildDataSize && (
        <Group heading="Storage">
          <Row title="Build Data" description={<p className="settings-description">DerivedData folders and simulators of your worktrees. Deleting a worktree deletes its share.</p>}>
            {buildDataSize}
          </Row>
        </Group>
      )}
    </>
  );
}

function Appearance() {
  const { theme, textPercent, terminalFontSize } = appearance.use();
  return (
    <>
      <Group heading="Window">
        <Row title="Theme" description={<p className="settings-description">System follows the macOS appearance.</p>}>
          <div className="segmented" role="radiogroup" aria-label="Theme">
            {themes.map(({ value, label }) => (
              <button key={value} type="button" role="radio" aria-checked={theme === value} onClick={() => setTheme(value)}>
                {label}
              </button>
            ))}
          </div>
        </Row>
        <Row title="Text size" description={<p className="settings-description">Scales all text in the window.</p>}>
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
        </Row>
      </Group>
      <Group heading="Terminal">
        <Row title="Font size" description={<p className="settings-description">The size of the terminal's text, in pixels.</p>}>
          <div className="settings-stepper">
            <button type="button" className="button-secondary settings-step" aria-label="Decrease terminal font size" onClick={() => changeTerminalFontSize(-1)}>
              <MinusIcon />
            </button>
            <span className="settings-value">{terminalFontSize}</span>
            <button type="button" className="button-secondary settings-step" aria-label="Increase terminal font size" onClick={() => changeTerminalFontSize(1)}>
              <PlusIcon />
            </button>
          </div>
        </Row>
      </Group>
    </>
  );
}

function Shortcuts() {
  const changes = shortcutChanges.use();
  const [recording, setRecording] = useState<string>();
  const [notice, setNotice] = useState("");

  const announce = (lost: string[]) => setNotice(lost.length ? `${lost.join(" and ")} lost ${lost.length > 1 ? "their shortcuts" : "its shortcut"}.` : "");

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
    <Group
      heading="Commands"
      notice={
        <p className="settings-description" role="status">
          {notice}
        </p>
      }
    >
      {commandTitles.map(({ id, title }) => (
        <Row key={id} title={title}>
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
        </Row>
      ))}
    </Group>
  );
}

// `buildDataSize` is the Build Data row's value, passed in by a window that has an Xcode project, so settings imports nothing from features/ios.
export function SettingsPage({ buildDataSize }: { buildDataSize: ReactNode }) {
  const [shown, setShown] = useState<SectionId>("general");
  const sheetOpen = sheet.use() !== null;
  const current = sections.find((s) => s.id === shown)!;
  return (
    <div className="sheet-backdrop" inert={sheetOpen} onMouseDown={closeSettings}>
      <div className="settings" role="dialog" aria-modal="true" aria-label="Settings" onMouseDown={(e) => e.stopPropagation()}>
        <nav className="settings-nav" aria-label="Settings sections">
          {sections.map(({ id, title, icon }) => (
            <button key={id} type="button" className="settings-nav-item" aria-current={id === shown ? "page" : undefined} onClick={() => setShown(id)}>
              {icon}
              {title}
            </button>
          ))}
        </nav>
        <div className="settings-main">
          <header className="settings-header">
            <h2 className="settings-breadcrumb">
              <span className="settings-breadcrumb-root">Settings</span>
              <span className="settings-breadcrumb-separator" aria-hidden="true">
                /
              </span>
              <span>{current.title}</span>
            </h2>
            <button type="button" className="icon-button" aria-label="Close Settings" title={tooltip("settings.close")} onClick={closeSettings}>
              <CloseIcon />
            </button>
          </header>
          <div className="settings-content">
            {shown === "general" && <General buildDataSize={buildDataSize} />}
            {shown === "appearance" && <Appearance />}
            {shown === "shortcuts" && <Shortcuts />}
          </div>
        </div>
      </div>
    </div>
  );
}
