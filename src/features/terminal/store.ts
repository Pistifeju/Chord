// Each worktree's xterm, wired to its shell over IPC and kept with its scrollback while hidden.
import { Channel } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { FitAddon } from "@xterm/addon-fit";
import { SearchAddon, type ISearchResultChangeEvent } from "@xterm/addon-search";
import { WebglAddon } from "@xterm/addon-webgl";
import { Terminal, type ITheme } from "@xterm/xterm";
import { commands, type TerminalEvent } from "../../ipc/bindings";
import { showBottomPanel } from "../bottompanel/store";
import { appearance } from "../settings/store";
import { fail } from "../../ui/alert";
import { openLink } from "../browser/store";
import { closeTerminalFind } from "./find";
import { isOverTerminal } from "./drop";
import { takesFocusOnShow } from "./focus";
import { droppedPathsText } from "./paths";
import { opaqueRgb, tokenValue } from "../../ui/tokenColor";
import { webUrlsIn } from "../../ui/webUrls";

type Shell = { term: Terminal; fit: FitAddon; search: SearchAddon; host: HTMLDivElement };

const shells = new Map<string, Shell>();

const ansiColors = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"].flatMap((name) => [name, `bright${name[0].toUpperCase()}${name.slice(1)}`]);

const theme = (): ITheme => ({
  background: tokenValue("canvas"),
  foreground: tokenValue("ink"),
  cursor: tokenValue("ink"),
  cursorAccent: tokenValue("canvas"),
  selectionBackground: tokenValue("editor-selection"),
  scrollbarSliderBackground: tokenValue("ink-disabled"),
  scrollbarSliderHoverBackground: tokenValue("ink-muted"),
  scrollbarSliderActiveBackground: tokenValue("ink-muted"),
  ...Object.fromEntries(ansiColors.map((name) => [name, tokenValue(`term-${name.replace("bright", "bright-").toLowerCase()}`)])),
});

// Resizing the WebGL canvas clears it and xterm repaints a frame later, so a live resize flashes a blank terminal; this paints in the same frame.
// shortcut: xterm 6's private render service, a no-op if it moves; recheck after an xterm upgrade.
const paintNow = (term: Terminal) => (term as any)._core?._renderService?._renderDebouncer?._innerRefresh?.();

// xterm takes a match background only as #RRGGBB, so the translucent match color is laid over the canvas first.
const opaque = (color: string): string => `#${opaqueRgb(color).map((v) => v.toString(16).padStart(2, "0")).join("")}`;

const matchColors = () => {
  const match = opaque(tokenValue("editor-match"));
  const current = tokenValue("accent-ink");
  return { matchBackground: match, matchOverviewRuler: match, activeMatchBackground: match, activeMatchBorder: current, activeMatchColorOverviewRuler: current };
};

/** Finds `query` in the worktree's terminal: `by` 0 keeps the current match while it still matches, 1 and -1 step to the next or previous. */
export function findInTerminal(worktree: string, query: string, by: 0 | 1 | -1) {
  const search = shells.get(worktree)?.search;
  if (!query) return search?.clearDecorations();
  const options = { decorations: matchColors() };
  if (by === -1) search?.findPrevious(query, options);
  else search?.findNext(query, { ...options, incremental: by === 0 });
}

export const onTerminalFindResults = (worktree: string, listener: (results: ISearchResultChangeEvent) => void) => shells.get(worktree)?.search.onDidChangeResults(listener);

export const endTerminalFind = (worktree: string) => shells.get(worktree)?.search.clearDecorations();

export const focusTerminal = (worktree: string) => shells.get(worktree)?.term.focus();

function fitNow({ term, fit, host }: Shell) {
  fit.fit();
  // A screen with no scrollback that the cursor has not filled grows and shrinks at its bottom, so its text holds to the top edge, which moves with the drag.
  host.toggleAttribute("data-top", term.buffer.active.baseY === 0 && term.buffer.active.cursorY < term.rows - 1);
  paintNow(term);
}

// The shell is gone: its terminal goes, and the panel closes.
function forget(worktree: string) {
  closeTerminalFind(worktree);
  shells.get(worktree)?.term.dispose();
  shells.delete(worktree);
  showBottomPanel(worktree, false);
}

// A URL can wrap onto the next rows, and a wide character fills two cells but one string unit, so each unit keeps its own cell.
function wrappedLine(term: Terminal, row: number) {
  const buffer = term.buffer.active;
  let first = row;
  while (first > 0 && buffer.getLine(first)?.isWrapped) first--;
  let last = row;
  while (buffer.getLine(last + 1)?.isWrapped) last++;
  const cell = buffer.getNullCell();
  let text = "";
  const cells: { x: number; y: number; lastX: number }[] = [];
  for (let y = first; y <= last; y++) {
    const bufferLine = buffer.getLine(y);
    for (let x = 0; x < term.cols; x++) {
      if (!bufferLine?.getCell(x, cell) || cell.getWidth() === 0) continue;
      const chars = cell.getChars() || " ";
      text += chars;
      for (let unit = 0; unit < chars.length; unit++) cells.push({ x: x + 1, y: y + 1, lastX: x + cell.getWidth() });
    }
  }
  return { text, cells };
}

function start(worktree: string, host: HTMLDivElement): Shell {
  const term = new Terminal({ fontFamily: tokenValue("font-code"), fontSize: appearance.get().terminalFontSize, scrollback: 10000, theme: theme(), allowProposedApi: true });
  const fit = new FitAddon();
  term.loadAddon(fit);
  const search = new SearchAddon();
  term.loadAddon(search);
  // ⌘ keys go to the app's shortcuts and the browser's copy and paste; every other key belongs to the shell.
  term.attachCustomKeyEventHandler((e) => !e.metaKey);
  host.addEventListener("keydown", (e) => {
    if (!e.metaKey) e.stopPropagation();
  });
  term.registerLinkProvider({
    provideLinks(line, callback) {
      const { text, cells } = wrappedLine(term, line - 1);
      const links = webUrlsIn(text).map(({ url, start }) => ({
        text: url,
        range: { start: { x: cells[start].x, y: cells[start].y }, end: { x: cells[start + url.length - 1].lastX, y: cells[start + url.length - 1].y } },
        // A plain click stays free for the shell's own mouse use and text selection.
        activate: (event: MouseEvent) => {
          if (event.metaKey) openLink(url);
        },
      }));
      callback(links.filter(({ range }) => range.start.y <= line && line <= range.end.y));
    },
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
  return { term, fit, search, host };
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
  // The observer runs after layout and before paint, at most once a frame, so the fit lands in the frame the panel resized in.
  const resized = new ResizeObserver(() => fitNow(shown));
  resized.observe(container);
  const unlistenDrop = getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type !== "drop" || payload.paths.length === 0) return;
    if (isOverTerminal(payload.position)) {
      term.paste(droppedPathsText(payload.paths));
      term.focus();
    }
  });
  if (takesFocusOnShow(worktree)) term.focus();
  return () => {
    resized.disconnect();
    void unlistenDrop.then((stop) => stop());
    themed.disconnect();
    unsubscribe();
    webgl?.dispose();
    host.remove();
  };
}
