// The chat of one pane: transcript, composer, and a Start button while no agent runs.
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { memo, useEffect, useLayoutEffect, useRef, useState, type ClipboardEvent, type KeyboardEvent } from "react";
import { tooltip } from "../../app/commands";
import type { Image, SlashCommand } from "../../ipc/bindings";
import { ArrowDownIcon, ArrowUpIcon, ChevronIcon, CloseIcon, PaperclipIcon } from "../../ui/icons";
import { imageMime, imageUrl, isImageType, type Attachment } from "./attachments";
import { DialogMenu, DialogPanel } from "./DialogPanel";
import { modelLabels } from "./model";
import { appearance } from "../settings/store";
import { StatusMark } from "../status/StatusMark";
import { FindBar } from "./FindBar";
import { finds } from "./find";
import { Markdown } from "./MarkdownView";
import { Lightbox } from "./Lightbox";
import { SlashMenu } from "./SlashMenu";
import { steppedIndex } from "./dialog";
import { completion, slashMenu, slashQuery } from "./slash";
import { closeLightbox, lightboxes, openLightbox, stepLightbox } from "./viewing";
import { answerDialog, attachFiles, attachments, attachPasted, chatOf, chats, closeSlash, drafts, openPicker, pickAttachments, removeAttachment, sendMessage, setDraft, slashClosed, startChat } from "./store";
import { isStreaming, shownSteps, stepImages, stepsSummary, type Item, type Message, type StepGroup } from "./transcript";
import "./chat.css";

function StartButton({ worktree, pane }: { worktree: string; pane: string }) {
  return (
    <button type="button" className="button-primary" onClick={() => startChat(worktree, pane)}>
      Start
    </button>
  );
}

const VERBS = ["Cooking", "Locking in", "Hold my beer", "Going feral"];

// The start time picks the verb, so a remount keeps it.
function Working({ since }: { since: number }) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  const s = Math.max(0, Math.floor((now - since) / 1000));
  return (
    <div className="chat-working">
      <StatusMark kind="running" />
      <span className="chat-working-verb">{VERBS[since % VERBS.length]}</span>
      <span>{s < 60 ? `${s}s` : `${Math.floor(s / 60)}m ${s % 60}s`}</span>
      <span>esc stops</span>
    </div>
  );
}

function Thumbs({ pane, images }: { pane: string; images: Image[] }) {
  return (
    <div className="chat-thumbs">
      {images.map((image, i) => (
        <button key={i} type="button" className="chat-thumb" aria-label={`Open image ${i + 1} of ${images.length}`} onClick={() => openLightbox(pane, images, i)}>
          <img src={imageUrl(image)} alt="" />
        </button>
      ))}
    </div>
  );
}

const MessageRow = memo(function MessageRow({ pane, message, fresh }: { pane: string; message: Message; fresh: boolean }) {
  const images = message.images ?? [];
  return (
    <div className="chat-message" data-role={message.role} data-fresh={fresh || undefined}>
      {message.role === "user" ? message.text : <Markdown text={message.text} />}
      {images.length > 0 && <Thumbs pane={pane} images={images} />}
    </div>
  );
});

const StepsRow = memo(function StepsRow({ pane, group, running, fresh }: { pane: string; group: StepGroup; running: boolean; fresh: boolean }) {
  const [expanded, setExpanded] = useState(false);
  const shown = shownSteps(group.steps, running, expanded);
  const images = stepImages(group.steps);
  return (
    <div className="chat-steps" data-fresh={fresh || undefined}>
      <button type="button" className="chat-steps-toggle" aria-expanded={expanded} onClick={() => setExpanded(!expanded)}>
        <span className="turn-chevron" data-open={expanded}>
          <ChevronIcon />
        </span>
        {stepsSummary(group.steps)}
      </button>
      {shown.map((step) =>
        step.kind === "thought" ? (
          <p key={`thought-${step.id}`} className="chat-step-thought">
            {step.text}
          </p>
        ) : (
          <div key={step.id} className="chat-step-call" data-state={step.state}>
            <span className="chat-step-title">{step.title}</span>
            <span className="chat-step-detail">{step.detail}</span>
          </div>
        ),
      )}
      {images.length > 0 && <Thumbs pane={pane} images={images} />}
    </div>
  );
});

// A streaming reply renders nothing, so its chunks leave the rows as they are.
type Rows = { items: Item[]; running: boolean; queued: string[]; /** How many items are the session's history. */ historyLength: number };
const sameRows = (a: Rows, b: Rows) =>
  a.running === b.running && a.historyLength === b.historyLength && a.queued === b.queued && a.items.length === b.items.length && a.items.every((item, i) => item === b.items[i] || (isStreaming(item) && isStreaming(b.items[i])));

const Messages = memo(function Messages({ pane, items, running, queued, historyLength }: { pane: string } & Rows) {
  // Only items that arrive live while the pane is open animate in; reopening a long chat and a session's history stay still.
  const loaded = useRef(items.length);
  // A session that opens with fewer items than the pane held: what arrives after its history is live.
  useLayoutEffect(() => {
    loaded.current = Math.min(loaded.current, items.length);
  });
  const still = Math.max(loaded.current, historyLength);
  return (
    <div className="chat-messages" data-selectable>
      {items.map((item, i) =>
        item.role === "steps" ? (
          <StepsRow key={i} pane={pane} group={item} running={running && i === items.length - 1} fresh={i >= still} />
        ) : item.role === "stopped" ? (
          <div key={i} className="chat-stopped" data-fresh={i >= still || undefined}>
            Stopped
          </div>
        ) : item.streaming ? null : (
          <MessageRow key={i} pane={pane} message={item} fresh={i >= still} />
        ),
      )}
      {queued.map((text, i) => (
        <div key={`queued-${i}`} className="chat-message" data-role="user" data-queued data-fresh title="Queued">
          {text}
        </div>
      ))}
    </div>
  );
}, sameRows);

const noAttachments: Attachment[] = [];

function fitComposer(el: HTMLTextAreaElement) {
  el.style.height = "auto";
  el.style.height = `${el.scrollHeight}px`;
}

export const ChatPane = memo(function ChatPane({ worktree, pane, focused }: { worktree: string; pane: string; focused: boolean }) {
  const chat = chats.usePart((all) => chatOf(all, pane));
  const draft = drafts.usePart((all) => all[pane] ?? "");
  const find = finds.usePart((all) => all[pane]);
  const viewing = lightboxes.usePart((all) => all[pane]);
  const closedOn = slashClosed.usePart((all) => all[pane]);
  const waiting = attachments.usePart((all) => all[pane]) ?? noAttachments;
  const root = useRef<HTMLDivElement>(null);
  const [dropping, setDropping] = useState(false);
  const composer = useRef<HTMLTextAreaElement>(null);
  const transcript = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);
  const [away, setAway] = useState(false);
  const [slashActive, setSlashActive] = useState(0);
  const ready = chat.agent === "ready";
  const hasChat = chat.capabilities === null || chat.capabilities.includes("chat");
  const canAttach = chat.capabilities?.includes("attachments") === true && chat.agent !== "idle";
  const steers = chat.running && chat.capabilities?.includes("steer") === true;
  const slashRows = slashMenu(chat.commands, draft, closedOn);
  const slashAt = Math.min(slashActive, slashRows.length - 1);
  const labels = modelLabels(chat);
  const canSend = ready && (!chat.running || steers);
  const textPercent = appearance.usePart((a) => a.textPercent);
  const dialogOpen = chat.dialog !== null;
  const menu = chat.dialog?.ask.kind === "menu" ? { dialog: chat.dialog, ask: chat.dialog.ask } : null;
  const composerShown = chat.agent !== "idle" && (!chat.dialog || menu !== null);

  // The pane under the pointer takes the dropped images.
  useEffect(() => {
    if (!canAttach) return;
    let carriesImages = false;
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "leave") return setDropping(false);
      if (payload.type === "enter") carriesImages = payload.paths.some((path) => imageMime(path));
      const rect = root.current?.getBoundingClientRect();
      const x = payload.position.x / window.devicePixelRatio;
      const y = payload.position.y / window.devicePixelRatio;
      const inside = !!rect && x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom;
      setDropping(carriesImages && inside && payload.type !== "drop");
      if (payload.type === "drop" && inside) attachFiles(pane, payload.paths);
    });
    return () => void unlisten.then((stop) => stop());
  }, [canAttach, pane]);

  // The composer grows with its draft up to the CSS max-height, then scrolls inside.
  useLayoutEffect(() => {
    if (composer.current) fitComposer(composer.current);
  }, [draft, textPercent, composerShown]);

  // A new pane width wraps the draft anew.
  useEffect(() => {
    const el = composer.current;
    if (!el) return;
    let width = el.clientWidth;
    const observer = new ResizeObserver(() => {
      if (el.clientWidth === width) return;
      width = el.clientWidth;
      fitComposer(el);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [composerShown]);

  useEffect(() => {
    if (ready && focused && !dialogOpen) composer.current?.focus();
  }, [ready, focused, dialogOpen]);
  const togglePicker = (command: "/model" | "/thinking") => (menu ? answerDialog(pane, menu.dialog.id, { kind: "cancelled" }) : openPicker(pane, command));

  // Follow the reply while the reader sits at the bottom; reading further up pauses it. A session that opens starts the transcript over and shows its end.
  const firstItem = useRef(chat.items[0]);
  useLayoutEffect(() => {
    const el = transcript.current;
    if (chat.items[0] !== firstItem.current) pinned.current = true;
    firstItem.current = chat.items[0];
    if (el && pinned.current) el.scrollTop = el.scrollHeight;
  }, [chat.items, chat.running, chat.queued]);

  const jumpToEnd = () => {
    const el = transcript.current;
    if (!el) return;
    pinned.current = true;
    const smooth = !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    el.scrollTo({ top: el.scrollHeight, behavior: smooth ? "smooth" : "auto" });
  };

  const send = () => {
    const text = draft.trim();
    if (!canSend || (!text && waiting.length === 0)) return;
    setDraft(pane, "");
    sendMessage(pane, text);
    composer.current?.focus();
  };

  const pickSlash = (command: SlashCommand) => {
    setDraft(pane, completion(command));
    composer.current?.focus();
  };

  const onComposerKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (slashRows.length > 0 && !e.nativeEvent.isComposing) {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        return setSlashActive(steppedIndex(slashAt, e.key === "ArrowDown" ? 1 : -1, slashRows.length));
      }
      // Return sends a command typed in full; Tab always completes.
      const picked = slashRows[slashAt];
      if (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey && draft !== `/${picked.name}`)) {
        e.preventDefault();
        return pickSlash(picked);
      }
    }
    if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      send();
    }
  };

  const onComposerPaste = (e: ClipboardEvent<HTMLTextAreaElement>) => {
    const files = [...e.clipboardData.files].filter((file) => isImageType(file.type));
    if (!canAttach || files.length === 0) return;
    e.preventDefault();
    files.forEach((file) => attachPasted(pane, file));
  };

  if (!hasChat) return null;

  return (
    <div
      className="chat"
      ref={root}
      onClick={(e) => {
        // A click outside a control lands in the composer, unless the reader selected text.
        if ((e.target as Element).closest("button, a, input, textarea")) return;
        if (window.getSelection()?.isCollapsed !== false) composer.current?.focus();
      }}
    >
      <div className="chat-body" data-running={chat.running || undefined}>
        <div
          className="chat-transcript"
          ref={transcript}
          onScroll={(e) => {
            const el = e.currentTarget;
            const fromEnd = el.scrollHeight - el.scrollTop - el.clientHeight;
            pinned.current = fromEnd < 24;
            setAway(fromEnd >= 72);
          }}
        >
          {chat.items.length === 0 && chat.agent === "idle" ? (
            <div className="chat-empty">
              <h1 className="chat-empty-title">No chat yet</h1>
              <p className="chat-empty-text">Start an agent in this worktree.</p>
              <StartButton worktree={worktree} pane={pane} />
            </div>
          ) : (
            <Messages pane={pane} items={chat.items} running={chat.running} queued={chat.queued} historyLength={chat.historyLength} />
          )}
        </div>
        <button type="button" className="chat-jump" data-shown={away || undefined} title="Jump to Latest" aria-label="Jump to Latest" onClick={jumpToEnd}>
          <ArrowDownIcon />
        </button>
        {find && <FindBar pane={pane} find={find} transcript={transcript} items={chat.items} />}
        {chat.running && <Working since={chat.since ?? Date.now()} />}
      </div>
      {viewing && <Lightbox images={viewing.images} index={viewing.index} onStep={(by) => stepLightbox(pane, by)} onClose={() => closeLightbox(pane)} />}
      {dropping && (
        <div className="drop-target chat-drop">
          <span className="drop-label">Drop images to attach</span>
        </div>
      )}
      {chat.notice && <p className="chat-notice">{chat.notice}</p>}
      {chat.agent === "idle" && chat.items.length > 0 && (
        <div className="chat-restart">
          <span className="chat-hint">The agent stopped.</span>
          <StartButton worktree={worktree} pane={pane} />
        </div>
      )}
      {chat.dialog && !menu && <DialogPanel key={chat.dialog.id} pane={pane} dialog={chat.dialog} />}
      {composerShown && (
        <div className="chat-composer-anchor">
          {menu && <DialogMenu key={menu.dialog.id} pane={pane} dialog={menu.dialog} options={menu.ask.options} current={menu.ask.current ?? null} filter={menu.ask.filter} />}
          {!menu && slashRows.length > 0 && <SlashMenu commands={slashRows} query={slashQuery(draft) ?? ""} active={slashAt} onActive={setSlashActive} onPick={pickSlash} onDismiss={() => closeSlash(pane, draft)} />}
          <div className="chat-composer">
            {waiting.length > 0 && (
              <div className="chat-attachments">
                {waiting.map((a, i) => (
                  <div key={a.id} className="chat-attachment">
                    <button type="button" className="chat-attachment-open" aria-label={`Open ${a.name}`} onClick={() => openLightbox(pane, waiting.map((w) => w.image), i)}>
                      <img src={imageUrl(a.image)} alt={a.name} />
                    </button>
                    <button type="button" className="chat-attachment-remove" title="Remove" aria-label={`Remove ${a.name}`} onClick={() => removeAttachment(pane, a.id)}>
                      <CloseIcon size={12} />
                    </button>
                  </div>
                ))}
              </div>
            )}
            <textarea
              ref={composer}
              className="chat-input"
              rows={1}
              disabled={!ready}
              value={draft}
              placeholder={steers ? "Steer the agent" : "Message the agent"}
              onChange={(e) => {
                setDraft(pane, e.target.value);
                setSlashActive(0);
              }}
              onKeyDown={onComposerKey}
              onPaste={onComposerPaste}
            />
            <div className="chat-composer-bar">
              {canAttach && (
                <button type="button" className="icon-button" title={tooltip("chat.attach")} aria-label="Attach Images" disabled={!ready} onClick={() => pickAttachments(pane)}>
                  <PaperclipIcon />
                </button>
              )}
              {labels && (
                <div className="chat-pickers">
                  <button type="button" className="chat-picker" title="Model" disabled={!ready} data-menu-toggle onClick={() => togglePicker("/model")}>
                    {labels.name}
                  </button>
                  {labels.thinking && (
                    <button type="button" className="chat-picker" title="Thinking Level" disabled={!ready} data-menu-toggle onClick={() => togglePicker("/thinking")}>
                      {labels.thinking}
                    </button>
                  )}
                </div>
              )}
              <button
                type="button"
                className="button-primary chat-send"
                title={steers ? "Steer" : "Send"}
                aria-label={steers ? "Steer" : "Send"}
                disabled={!canSend || (!draft.trim() && waiting.length === 0)}
                onClick={send}
              >
                <ArrowUpIcon />
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
});
