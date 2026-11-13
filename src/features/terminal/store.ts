// Each worktree's xterm, wired to its shell over IPC and kept with its scrollback while hidden.
import { Channel } from "@tauri-apps/api/core";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import { Terminal, type ITheme } from "@xterm/xterm";
import { commands, type TerminalEvent } from "../../ipc/bindings";
import { bottomPanelOf, bottomPanels, showBottomPanel } from "../bottompanel/store";
import { appearance } from "../settings/store";
import { fail } from "../worktrees/store";

type Shell = { term: Terminal; fit: FitAddon; host: HTMLDivElement };

const shells = new Map<string, Shell>();
// The worktree whose terminal takes the focus once it shows: the one ⌘T just opened.
let focusOnShow: string | null = null;

const ansiColors = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"].flatMap((name) => [name, `bright${name[0].toUpperCase()}${name.slice(1)}`]);

const token = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(`--${name}`).trim();

const theme = (): ITheme => ({
  background: token("canvas"),
  foreground: token("ink"),
  cursor: token("ink"),
  cursorAccent: token("canvas"),
  selectionBackground: token("editor-selection"),
  scrollbarSliderBackground: token("ink-disabled"),
  scrollbarSliderHoverBackground: token("ink-muted"),
  scrollbarSliderActiveBackground: token("ink-muted"),
  ...Object.fromEntries(ansiColors.map((name) => [name, token(`term-${name.replace("bright", "bright-").toLowerCase()}`)])),
});

// Resizing the WebGL canvas clears it and xterm repaints a frame later, so a live resize flashes a blank terminal; this paints in the same frame.
// shortcut: xterm 6's private render service, a no-op if it moves; recheck after an xterm upgrade.
const paintNow = (term: Terminal) => (term as any)._core?._renderService?._renderDebouncer?._innerRefresh?.();

function fitNow({ term, fit }: Shell) {
  fit.fit();
  paintNow(term);
}

export const hideTerminal = (worktree: string) => showBottomPanel(worktree, false);

export function toggleTerminal(worktree: string) {
  const open = !bottomPanelOf(bottomPanels.get(), worktree).open;
  focusOnShow = open ? worktree : null;
  showBottomPanel(worktree, open);
}

// The shell is gone: its terminal goes, and the panel closes.
function forget(worktree: string) {
  shells.get(worktree)?.term.dispose();
  shells.delete(worktree);
  showBottomPanel(worktree, false);
}

function start(worktree: string, host: HTMLDivElement): Shell {
  const term = new Terminal({ fontFamily: 'ui-monospace, "SF Mono", Menlo, monospace', fontSize: appearance.get().terminalFontSize, scrollback: 10000, theme: theme() });
  const fit = new FitAddon();
  term.loadAddon(fit);
  // ⌘ keys go to the app's shortcuts and the browser's copy and paste; every other key belongs to the shell.
  term.attachCustomKeyEventHandler((e) => !e.metaKey);
  host.addEventListener("keydown", (e) => {
    if (!e.metaKey) e.stopPropagation();
  });
  term.open(host);
  fit.fit();
  const channel = new Channel<TerminalEvent>();
  channel.onmessage = (event) => (event.kind === "output" ? term.write(event.text) : forget(worktree));
  // Each call runs on its own Rust thread, so they queue here: input stays in order and nothing reaches the shell before it starts.
  let started = true;
  let calls: Promise<unknown> = commands.startTerminal(worktree, term.cols, term.rows, channel).catch((error) => {
    started = false;
    forget(worktree);
    fail(error);
  });
  const send = (call: () => Promise<unknown>) => {
    calls = calls.then(() => started && call()).catch(console.error);
  };
  term.onData((text) => send(() => commands.writeTerminal(worktree, text)));
  term.onResize(({ cols, rows }) => send(() => commands.resizeTerminal(worktree, cols, rows)));
  return { term, fit, host };
}

/** Shows the worktree's terminal in `container`, starting its shell the first time; the returned function hides it again. */
export function showTerminal(worktree: string, container: HTMLElement): () => void {
  let shell = shells.get(worktree);
  if (shell) {
    container.append(shell.host);
  } else {
    const host = document.createElement("div");
    host.className = "terminal-host";
    container.append(host);
    shell = start(worktree, host);
    shells.set(worktree, shell);
  }
  const { term, host } = shell;
  const shown = shell;
  let webgl: WebglAddon | null = new WebglAddon();
  try {
    term.loadAddon(webgl);
    webgl.onContextLoss(() => {
      webgl?.dispose();
      webgl = null;
    });
  } catch {
    // Without WebGL xterm draws with the DOM.
    webgl = null;
  }
  // A hidden terminal misses theme and font changes, so it takes them as it shows.
  const applyAppearance = () => {
    term.options.theme = theme();
    term.options.fontSize = appearance.get().terminalFontSize;
    fitNow(shown);
  };
  applyAppearance();
  const themed = new MutationObserver(applyAppearance);
  themed.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  const unsubscribe = appearance.subscribe(applyAppearance);
  // One fit per frame: each fit reflows the scrollback and resizes the shell's terminal.
  let frame = 0;
  const resized = new ResizeObserver(() => {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => fitNow(shown));
  });
  resized.observe(container);
  if (focusOnShow === worktree) {
    focusOnShow = null;
    term.focus();
  }
  return () => {
    cancelAnimationFrame(frame);
    resized.disconnect();
    themed.disconnect();
    unsubscribe();
    webgl?.dispose();
    host.remove();
  };
}
