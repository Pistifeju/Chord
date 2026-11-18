// A browser tab's view: back, forward, reload, the address field and Open in Browser over the page, which Rust shows as a web view of its own in the space below and hides while a sheet, Settings or a tab drag covers the window.
import { useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import type { BrowserRect } from "../../ipc/bindings";
import { sheet } from "../../app/sheet";
import { ArrowLeftIcon, ArrowRightIcon, OpenOutsideIcon, ReloadIcon } from "../../ui/icons";
import { settingsOpen } from "../settings/store";
import { webAddress } from "./address";
import { browse, hidePage, loading, movePage, openInDefaultBrowser, showPage } from "./store";
import "./browser.css";

const rectOf = (element: Element): BrowserRect => {
  const { left, top, width, height } = element.getBoundingClientRect();
  return { x: left, y: top, width, height };
};

const same = (a: BrowserRect | null, b: BrowserRect) => a !== null && a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;

export function BrowserView({ tab, url, dragging }: { tab: string; url: string; dragging: boolean }) {
  const area = useRef<HTMLDivElement>(null);
  const shownRect = useRef<BrowserRect | null>(null);
  const [draft, setDraft] = useState<string | null>(null);
  const busy = loading.usePart((all) => all[tab] ?? false);
  const covered = sheet.use() !== null || settingsOpen.use() || dragging;
  // The page loads its first address once; later addresses come from the page itself.
  const firstUrl = useRef(url);

  useLayoutEffect(() => {
    const element = area.current;
    if (!element || covered) return;
    shownRect.current = rectOf(element);
    showPage(tab, firstUrl.current, shownRect.current);
    const measure = () => {
      const next = rectOf(element);
      if (same(shownRect.current, next)) return;
      shownRect.current = next;
      movePage(tab, next);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    window.addEventListener("resize", measure);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", measure);
      hidePage(tab);
    };
  }, [tab, covered]);

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Escape") {
      setDraft(null);
      e.currentTarget.blur();
    }
    if (e.key !== "Enter" || draft === null) return;
    const next = webAddress(draft);
    if (!next) return;
    browse(tab, { kind: "open", url: next });
    setDraft(null);
    e.currentTarget.blur();
  };

  return (
    <div className="browser">
      <div className="browser-bar">
        <button type="button" className="icon-button" title="Back" aria-label="Back" onClick={() => browse(tab, { kind: "back" })}>
          <ArrowLeftIcon />
        </button>
        <button type="button" className="icon-button" title="Forward" aria-label="Forward" onClick={() => browse(tab, { kind: "forward" })}>
          <ArrowRightIcon />
        </button>
        <button type="button" className="icon-button" title="Reload" aria-label="Reload" aria-busy={busy} onClick={() => browse(tab, { kind: "reload" })}>
          <ReloadIcon />
        </button>
        <input
          className="input browser-address"
          aria-label="Address"
          spellCheck={false}
          value={draft ?? url}
          onChange={(e) => setDraft(e.target.value)}
          onFocus={(e) => e.target.select()}
          onBlur={() => setDraft(null)}
          onKeyDown={onKeyDown}
          data-selectable
        />
        <button type="button" className="icon-button" title="Open in Default Browser" aria-label="Open in Default Browser" onClick={() => openInDefaultBrowser(url)}>
          <OpenOutsideIcon />
        </button>
      </div>
      <div className="browser-page" ref={area} />
    </div>
  );
}
