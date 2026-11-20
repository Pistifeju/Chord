// The Settings panel over the blurred window: a section list on the left, the chosen section's grouped rows on the right.
import { useEffect, useState, type KeyboardEvent, type ReactNode } from "react";
import type { Theme } from "../../ipc/bindings";
import { changeShortcut, commandTitles, resetShortcut, shortcutLabel, tooltip } from "../../app/commands";
import { sheet } from "../../app/sheet";
import { recordedShortcut } from "../../app/shortcuts";
import { dragWindowOrClose } from "../../ui/backdrop";
import { Row } from "../../ui/Row";
import { onOff, Segmented } from "../../ui/Segmented";
import { CheckAgain, HarnessRows } from "../harnesses/HarnessRows";
import { rowsNote } from "../harnesses/harnessCheck";
import { harnessChecks, watchHarnessChecks } from "../harnesses/store";
import { AppearanceIcon, ChatIcon, CloseIcon, KeyboardIcon, MinusIcon, PhoneIcon, PlusIcon, SlidersIcon } from "../../ui/icons";
import { appearance, changeFontSize, changeTextSize, closeSettings, keepAwake, linksInDefaultBrowser, muteNotifications, setKeepAwake, setLinksInDefaultBrowser, setMuteNotifications, setTheme, settingsSection, shortcutChanges, type FontSizeSetting, type SettingsSection } from "./store";
import "./settings.css";

// The value is whether links open in the default browser.
const linkTargets = [
  { value: false, label: "Chord" },
  { value: true, label: "Default Browser" },
];

const themes: { value: Theme; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

const sections: { id: SettingsSection; title: string; icon: ReactNode }[] = [
  { id: "general", title: "General", icon: <SlidersIcon /> },
  { id: "agents", title: "Agents", icon: <ChatIcon /> },
  { id: "appearance", title: "Appearance", icon: <AppearanceIcon /> },
  { id: "shortcuts", title: "Shortcuts", icon: <KeyboardIcon /> },
  { id: "simulators", title: "Simulators", icon: <PhoneIcon /> },
];

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
  const muted = muteNotifications.use();
  const outside = linksInDefaultBrowser.use();
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
          <Segmented label="Keep the Mac awake while an agent runs" options={onOff} value={enabled} onChange={(value) => setKeepAwake(value).then((failure) => setError(failure ?? ""))} />
        </Row>
        <Row title="Notifications" description={<p className="settings-description">A macOS notification when an agent finishes or waits for you.</p>}>
          <Segmented label="Notify when an agent finishes or waits" options={onOff} value={!muted} onChange={(value) => setMuteNotifications(!value)} />
        </Row>
      </Group>
      <Group heading="Links">
        <Row title="Open links in" description={<p className="settings-description">Web links from chats and the terminal open in a Chord tab or in your default browser.</p>}>
          <Segmented label="Open links in" options={linkTargets} value={outside} onChange={setLinksInDefaultBrowser} />
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

// The rows draw their own card, so the section skips `Group`.
function Agents() {
  useEffect(watchHarnessChecks, []);
  const checks = harnessChecks.usePart((s) => s.checks);
  const note = checks && rowsNote(checks);
  return (
    <section className="settings-section" aria-label="Agent for new sessions">
      <div className="settings-section-header">
        <h3 className="settings-heading">Agent for new sessions</h3>
        <CheckAgain />
      </div>
      <HarnessRows checks={checks} />
      {note && <p className="settings-description settings-note">{note}</p>}
    </section>
  );
}

function Appearance() {
  const { theme, textPercent } = appearance.use();
  return (
    <>
      <Group heading="Window">
        <Row title="Theme" description={<p className="settings-description">System follows the macOS appearance.</p>}>
          <Segmented label="Theme" options={themes} value={theme} onChange={setTheme} />
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
      <Group heading="Editor">
        <FontSizeRow setting="editorFontSize" area="editor" description="The size of code in the editor and in diffs, in pixels." />
      </Group>
      <Group heading="Terminal">
        <FontSizeRow setting="terminalFontSize" area="terminal" description="The size of the terminal's text, in pixels." />
      </Group>
    </>
  );
}

function FontSizeRow({ setting, area, description }: { setting: FontSizeSetting; area: string; description: string }) {
  const size = appearance.use()[setting];
  return (
    <Row title="Font size" description={<p className="settings-description">{description}</p>}>
      <div className="settings-stepper">
        <button type="button" className="button-secondary settings-step" aria-label={`Decrease ${area} font size`} onClick={() => changeFontSize(setting, -1)}>
          <MinusIcon />
        </button>
        <span className="settings-value">{size}</span>
        <button type="button" className="button-secondary settings-step" aria-label={`Increase ${area} font size`} onClick={() => changeFontSize(setting, 1)}>
          <PlusIcon />
        </button>
      </div>
    </Row>
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

function Simulators({ screenshots, services }: { screenshots: ReactNode; services: ReactNode }) {
  return (
    <>
      <Group heading="Screenshots and recordings">{screenshots}</Group>
      <Group heading="Background services" notice={<p className="settings-description">Applies at the next boot · iOS 18.5 and newer</p>}>
        {services}
      </Group>
    </>
  );
}

// `buildDataSize` is the Build Data row's value, `simulatorScreenshots` and `simulatorServices` the Simulators section's rows, passed in by a window that has an Xcode project, so settings imports nothing from features/ios.
export function SettingsPage({ buildDataSize, simulatorScreenshots, simulatorServices }: { buildDataSize: ReactNode; simulatorScreenshots: ReactNode; simulatorServices: ReactNode }) {
  const [shown, setShown] = useState(settingsSection.get);
  const sheetOpen = sheet.use() !== null;
  const shownSections = sections.filter((s) => s.id !== "simulators" || simulatorServices);
  const current = shownSections.find((s) => s.id === shown) ?? shownSections[0];
  return (
    <div className="sheet-backdrop" inert={sheetOpen} onMouseDown={(e) => dragWindowOrClose(e, closeSettings)}>
      <div className="settings" role="dialog" aria-modal="true" aria-label="Settings" onMouseDown={(e) => e.stopPropagation()}>
        <nav className="settings-nav" aria-label="Settings sections">
          {shownSections.map(({ id, title, icon }) => (
            <button key={id} type="button" className="settings-nav-item" aria-current={id === current.id ? "page" : undefined} onClick={() => setShown(id)}>
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
            {current.id === "general" && <General buildDataSize={buildDataSize} />}
            {current.id === "agents" && <Agents />}
            {current.id === "appearance" && <Appearance />}
            {current.id === "shortcuts" && <Shortcuts />}
            {current.id === "simulators" && <Simulators screenshots={simulatorScreenshots} services={simulatorServices} />}
          </div>
        </div>
      </div>
    </div>
  );
}
