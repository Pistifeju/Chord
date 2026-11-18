// Connects browser tabs to IPC: a web link opened in a browser tab of the open worktree or, as Settings chooses, in the default browser; each tab's page shown over its pane; its loads and title from Rust.
import { commands, events, type BrowserAction, type BrowserRect } from "../../ipc/bindings";
import { fail } from "../../ui/alert";
import { externalStore } from "../../ui/externalStore";
import { openBrowserTab, setBrowserPage } from "../panes/store";
import { linksInDefaultBrowser } from "../settings/store";
import { openWorktree } from "../worktrees/store";
import { webAddress } from "./address";

/** The browser tabs whose page is loading. */
export const loading = externalStore<Record<string, boolean>>({});

export const openInDefaultBrowser = (url: string) => void commands.openLink(url).catch(fail);

// Only web addresses open in a tab; a mail link, or a link while no worktree shows, goes to the default app.
export function openLink(url: string) {
  const worktree = openWorktree();
  const web = /^https?:\/\//i.test(url) ? webAddress(url) : null;
  if (linksInDefaultBrowser.get() || !worktree || !web) openInDefaultBrowser(url);
  else openBrowserTab(worktree, web);
}

export const showPage = (tab: string, url: string, rect: BrowserRect) => void commands.showBrowser(tab, url, rect).catch(fail);
export const movePage = (tab: string, rect: BrowserRect) => void commands.moveBrowser(tab, rect).catch(console.error);
export const hidePage = (tab: string) => void commands.hideBrowser(tab).catch(console.error);
export const browse = (tab: string, action: BrowserAction) => void commands.browse(tab, action).catch(fail);

export function watchBrowsers() {
  const pages = events.browserChanged.listen(({ payload: { tab, page } }) => {
    if (page.kind === "title") return setBrowserPage(tab, { title: page.title });
    loading.set({ ...loading.get(), [tab]: page.kind === "loading" });
    setBrowserPage(tab, { url: page.url });
  });
  const links = events.linkRequested.listen(({ payload }) => openLink(payload));
  return () => {
    void pages.then((stop) => stop());
    void links.then((stop) => stop());
  };
}
