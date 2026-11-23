// The chat of one pane: transcript, composer, and a Start button while no agent runs, or the harness rows while its harness cannot start.
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { flushSync } from "react-dom";
import { memo, useEffect, useLayoutEffect, useMemo, useRef, useState, type ClipboardEvent, type CSSProperties, type KeyboardEvent } from "react";
import { tooltip } from "../../app/commands";
import type { Checks, HarnessCheck, Image, SlashCommand, SubagentNode } from "../../ipc/bindings";
import { elementAtDrop } from "../../ui/dropPoint";
import { ArrowDownIcon, ArrowUpIcon, ChevronIcon, CloseIcon, EyeIcon, FileIcon, FolderIcon, ImageIcon, PaperclipIcon, PencilIcon, PlayIcon, SearchIcon } from "../../ui/icons";
import { imageUrl, isImageType, type Attachment } from "./attachments";
import { AgentMenu } from "./AgentMenu";
import { agentPickerPane, closeAgentPicker, openAgentPicker } from "./agentPicker";
import { DialogMenu, DialogPanel } from "./DialogPanel";
import { compactionLabel, contextMeter } from "./context";
import { useClock } from "./clock";
import { elapsedText } from "./elapsed";
import { modelLabels } from "./model";
import { PermissionIcon } from "./permissionIcons";
import { appearance } from "../settings/store";
import { StatusMark } from "../status/StatusMark";
import { HarnessRows } from "../harnesses/HarnessRows";
import { harnessMarks } from "../harnesses/logos";
import { blockingRows } from "../harnesses/harnessCheck";
import { checkHarnesses, harnessChecks, watchHarnessChecks } from "../harnesses/store";
import { TranscriptFind } from "./TranscriptFind";
import { finds } from "./find";
import { Markdown } from "./MarkdownView";
import { CopyButton } from "../../ui/CopyButton";
import { messageTime } from "./messageTime";
import { Lightbox } from "./Lightbox";
import { SlashMenu } from "./SlashMenu";
import { composerKey, composerMenu, composerState, droppedFiles, pickerClick, sendable } from "./composer";
import { isTicking, subagentLine, subagentSummary } from "./subagents";
import { completion, slashMenu, slashQuery } from "./slash";
import { closeLightbox, lightboxes, openLightbox, stepLightbox } from "./viewing";
import { addPathToDraft, answerDialog, attachFiles, attachments, attachPasted, chatOf, chats, closeSlash, drafts, openPicker, paneHarnesses, pathDropPane, pickAttachments, removeAttachment, sendMessage, setDraft, slashClosed, startChat, takesAttachments } from "./store";
import { callKind, callText, callTime, isLive, isStreaming, queuedImages, stepDoing, stepImages, stepsChanges, stepsHeadline, stepsSummary, stepsTime, type CallKind, type CompactionMark, type Item, type Message, type SteerImages, type Step, type StepGroup } from "./transcript";
import "./chat.css";

function SubagentLine({ mark, text }: { mark: SubagentNode["state"]; text: string }) {
  return (
    <>
      <StatusMark kind={mark} />
      <span className="chat-subagent-text">{text}</span>
    </>
  );
}

function SubagentLines({ nodes }: { nodes: SubagentNode[] }) {
  const now = useClock(isTicking(nodes));
  const summary = subagentSummary(nodes, now);
  const indent = summary ? 1 : 0;
  return (
    <section aria-label="Subagents" className="chat-subagents">
      {summary && (
        <div className="chat-subagent chat-subagent-summary">
          <SubagentLine mark={summary.mark} text={summary.text} />
        </div>
      )}
      <ul className="chat-subagent-list">
        {nodes.map((node) => (
          <li key={node.id} className="chat-subagent" style={{ "--depth": node.depth + indent } as CSSProperties}>
            <SubagentLine mark={node.state} text={subagentLine(node, now)} />
          </li>
        ))}
      </ul>
    </section>
  );
}

/** The rows the user acts on before a start works; only shown rows watch window focus, so ready panes start no checks. */
function useBlockingRows(pane: string, idle: boolean): { checks: Checks | null; rows: HarnessCheck[] } {
  const checks = harnessChecks.usePart((s) => (idle ? s.checks : null));
  const harness = paneHarnesses.usePart((all) => all[pane] ?? null);
  const rows = blockingRows(checks, harness);
  const shown = rows.length > 0;
  useEffect(() => {
    if (idle) void checkHarnesses();
  }, [idle]);
  useEffect(() => (shown ? watchHarnessChecks() : undefined), [shown]);
  return { checks, rows };
}

function Start({ worktree, pane, blocking }: { worktree: string; pane: string; blocking: { checks: Checks | null; rows: HarnessCheck[] } }) {
  if (blocking.rows.length > 0) return <HarnessRows checks={blocking.checks} rows={blocking.rows} />;
  return (
    <button type="button" className="button-primary" onClick={() => startChat(worktree, pane)}>
      Start
    </button>
  );
}

const VERBS = ["Cooking…", "Locking in…", "Hold my beer…", "Going feral…"];

// The start time picks the verb, so a remount keeps it.
function Working({ since, compacting }: { since: number; compacting: boolean }) {
  const now = useClock(true);
  return (
    <div className="chat-working">
      <StatusMark kind="running" />
      <span className="chat-working-verb">{compacting ? "Compacting…" : VERBS[since % VERBS.length]}</span>
      <span>{elapsedText(now - since)}</span>
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

// A long message folds to its first lines; an open find bar unfolds it so every match shows.
function UserText({ pane, text }: { pane: string; text: string }) {
  const el = useRef<HTMLDivElement>(null);
  const [long, setLong] = useState(false);
  const [open, setOpen] = useState(false);
  const finding = finds.usePart((all) => pane in all);
  const folded = !open && !finding;
  useLayoutEffect(() => {
    const node = el.current;
    if (!node || !folded) return;
    const measure = () => setLong(node.scrollHeight > node.clientHeight);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, [folded, text]);
  const toggle = () => {
    flushSync(() => setOpen(!open));
    if (open) el.current?.scrollIntoView({ block: "nearest" });
  };
  return (
    <>
      <div ref={el} className="chat-message-text" data-folded={folded || undefined} data-long={long || undefined}>
        {text}
      </div>
      {long && !finding && (
        <button type="button" className="chat-message-more" aria-expanded={open} onClick={toggle}>
          {open ? "Show less" : "Show more"}
        </button>
      )}
    </>
  );
}

const MessageRow = memo(function MessageRow({ pane, message, fresh }: { pane: string; message: Message; fresh: boolean }) {
  const images = message.images ?? [];
  return (
    <div className="chat-turn" data-role={message.role} data-fresh={fresh || undefined}>
      <div className="chat-message" data-role={message.role}>
        {message.role === "user" ? message.text && <UserText pane={pane} text={message.text} /> : <Markdown text={message.text} />}
        {images.length > 0 && <Thumbs pane={pane} images={images} />}
      </div>
      <div className="chat-message-meta">
        <CopyButton text={message.text} label="Copy Message" className="chat-message-copy" />
        {message.at !== undefined && <time title={new Date(message.at).toLocaleString()}>{messageTime(message.at, Date.now())}</time>}
      </div>
    </div>
  );
});

const stepIcons: Partial<Record<CallKind, typeof FileIcon>> = { edit: PencilIcon, write: PencilIcon, read: EyeIcon, search: SearchIcon, list: FolderIcon, run: PlayIcon };

function StepLine({ step, worktree, showChanges }: { step: Step; worktree: string; showChanges: boolean }) {
  if (step.kind === "thought") {
    return (
      <div className="chat-step" data-kind="thought">
        <span className="chat-step-text">{step.text}</span>
      </div>
    );
  }
  const changes = showChanges && step.state === "done" ? step.changes : undefined;
  const Icon = stepIcons[callKind(step.title)] ?? FileIcon;
  return (
    <div className="chat-step" data-state={step.state} title={step.detail}>
      <span className="chat-step-icon">
        <Icon />
      </span>
      <span className="chat-step-title">{step.title}</span>
      <span className="chat-step-text" data-code={step.command}>
        {callText(step, worktree)}
      </span>
      <span className="chat-step-end">
        {step.state === "running" && <StatusMark kind="running" />}
        {changes && changes.added > 0 && <span className="chat-steps-added">+{changes.added}</span>}
        {changes && changes.removed > 0 && <span className="chat-steps-removed">−{changes.removed}</span>}
        {callTime(step) && <span className="chat-step-time">{callTime(step)}</span>}
      </span>
    </div>
  );
}

const StepsRow = memo(function StepsRow({ pane, worktree, group, live, fresh }: { pane: string; worktree: string; group: StepGroup; live: boolean; fresh: boolean }) {
  const [expanded, setExpanded] = useState(false);
  const images = stepImages(group.steps);
  const total = stepsTime(group.steps);
  const changes = stepsChanges(group.steps);
  // One changed step would repeat the toggle line's counts.
  const changedSteps = group.steps.filter((step) => step.kind === "call" && step.state === "done" && step.changes).length;
  const last = group.steps[group.steps.length - 1];
  const line = live && last ? stepDoing(last) : stepsHeadline(group.steps);
  // The line the row mounted with shows still; a new line fades in, so a history group holds and a live one reads its steps.
  const firstLine = useRef(line);
  return (
    <div className="chat-steps" data-fresh={fresh || undefined}>
      <div className="chat-steps-head">
        <button type="button" className="chat-steps-toggle" aria-expanded={expanded} onClick={() => setExpanded(!expanded)}>
          <span className="turn-chevron" data-open={expanded}>
            <ChevronIcon />
          </span>
          {live && <StatusMark kind="running" />}
          <span key={line} className="chat-steps-summary" title={live ? undefined : stepsSummary(group.steps)} data-changed={line !== firstLine.current || undefined}>
            {line}
          </span>
          {!live && changes && changes.added > 0 && <span className="chat-steps-added">+{changes.added}</span>}
          {!live && changes && changes.removed > 0 && <span className="chat-steps-removed">−{changes.removed}</span>}
        </button>
        {!live && total && <span className="chat-step-time">{total}</span>}
      </div>
      {expanded && (
        <div className="chat-step-lines">
          {group.steps.map((step) => (
            <StepLine key={step.kind === "thought" ? `thought-${step.id}` : step.id} step={step} worktree={worktree} showChanges={changedSteps > 1} />
          ))}
        </div>
      )}
      {images.length > 0 && <Thumbs pane={pane} images={images} />}
    </div>
  );
});

const CompactionRow = memo(function CompactionRow({ mark, fresh }: { mark: CompactionMark; fresh: boolean }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div className="chat-steps" data-fresh={fresh || undefined}>
      <div className="chat-steps-head">
        <button type="button" className="chat-steps-toggle" aria-expanded={expanded} onClick={() => setExpanded(!expanded)}>
          <span className="turn-chevron" data-open={expanded}>
            <ChevronIcon />
          </span>
          <span className="chat-steps-summary">{compactionLabel(mark.tokensBefore)}</span>
        </button>
      </div>
      {expanded && (
        <div className="chat-compaction-summary">
          <Markdown text={mark.summary} />
        </div>
      )}
    </div>
  );
});

// A streaming reply renders nothing, so its chunks leave the rows as they are.
type Rows = { worktree: string; running: boolean; items: Item[]; queued: string[]; steerImages: SteerImages[]; subagents: SubagentNode[]; subagentsAt: number | null; /** How many items are the session's history. */ historyLength: number };
const sameItems = (a: Item[], b: Item[]) => a === b || (a.length === b.length && a.every((item, i) => item === b[i] || (isStreaming(item) && isStreaming(b[i]))));
const sameRows = (a: Rows, b: Rows) => a.worktree === b.worktree && a.running === b.running && a.historyLength === b.historyLength && a.queued === b.queued && a.steerImages === b.steerImages && a.subagents === b.subagents && a.subagentsAt === b.subagentsAt && sameItems(a.items, b.items);

const Messages = memo(function Messages({ pane, worktree, running, items, queued, steerImages, subagents, subagentsAt, historyLength }: { pane: string } & Rows) {
  // Only items that arrive live while the pane is open animate in; reopening a long chat and a session's history stay still.
  const loaded = useRef(items.length);
  // A session that opens with fewer items than the pane held: what arrives after its history is live.
  useLayoutEffect(() => {
    loaded.current = Math.min(loaded.current, items.length);
  });
  const still = Math.max(loaded.current, historyLength);
  const rows = items.map((item, i) =>
    item.role === "steps" ? (
      <StepsRow key={i} pane={pane} worktree={worktree} group={item} live={isLive(items, i, running)} fresh={i >= still} />
    ) : item.role === "compaction" ? (
      <CompactionRow key={i} mark={item} fresh={i >= still} />
    ) : item.role === "stopped" ? (
      <div key={i} className="chat-stopped" data-fresh={i >= still || undefined}>
        Stopped
      </div>
    ) : item.streaming ? null : (
      <MessageRow key={i} pane={pane} message={item} fresh={i >= still} />
    ),
  );
  if (subagents.length > 0) rows.splice(Math.min(subagentsAt ?? items.length, items.length), 0, <SubagentLines key="subagents" nodes={subagents} />);
  return (
    <div className="chat-messages" data-selectable>
      {rows}
      {queued.map((text, i) => {
        const images = queuedImages(steerImages, text);
        return (
          <div key={`queued-${i}`} className="chat-message" data-role="user" data-queued data-fresh title="Queued">
            {text}
            {images.length > 0 && <Thumbs pane={pane} images={images} />}
          </div>
        );
      })}
    </div>
  );
}, sameRows);

const noAttachments: Attachment[] = [];
const noSubagents: SubagentNode[] = [];

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
  const pathOver = pathDropPane.usePart((over) => over === pane);
  const root = useRef<HTMLDivElement>(null);
  const [dropping, setDropping] = useState(false);
  const composer = useRef<HTMLTextAreaElement>(null);
  const transcript = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);
  const [away, setAway] = useState(false);
  const [slashActive, setSlashActive] = useState(0);
  const { ready, steers, canSend, menu, shown: composerShown } = composerState(chat);
  const hasChat = chat.capabilities === null || chat.capabilities.includes("chat");
  const blocking = useBlockingRows(pane, chat.agent === "idle");
  const blocked = blocking.rows.length > 0;
  const canAttach = takesAttachments(chat);
  const slashRows = slashMenu(chat.commands, draft, closedOn);
  const slashAt = Math.min(slashActive, slashRows.length - 1);
  const labels = modelLabels(chat);
  const context = contextMeter(chat);
  const canSubmit = sendable(canSend, draft, waiting.length);
  const textPercent = appearance.usePart((a) => a.textPercent);
  const dialogOpen = chat.dialog !== null;
  const agentAsked = agentPickerPane.usePart((open) => open === pane);
  const harness = paneHarnesses.usePart((all) => all[pane] ?? null);
  const floating = composerMenu(menu !== null, agentAsked && ready && harness !== null, slashRows.length > 0);
  const agentButton = useRef<HTMLButtonElement>(null);
  const AgentLogo = harness ? harnessMarks[harness]?.Logo : undefined;

  useEffect(() => {
    if (!ready && agentPickerPane.get() === pane) closeAgentPicker();
  }, [ready, pane]);

  // The pane under the pointer takes the dropped images; any other file, such as a video, goes into the draft as its path.
  useEffect(() => {
    let carriesImages = false;
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "leave") return setDropping(false);
      if (payload.type === "enter") carriesImages = droppedFiles(payload.paths, canAttach).attach.length > 0;
      const inside = !!root.current?.contains(elementAtDrop(payload.position));
      setDropping(carriesImages && inside && payload.type !== "drop");
      if (payload.type !== "drop" || !inside) return;
      const dropped = droppedFiles(payload.paths, canAttach);
      attachFiles(pane, dropped.attach);
      for (const path of dropped.draft) addPathToDraft(pane, path);
      if (dropped.draft.length > 0) composer.current?.focus();
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
  const closeMenus = () => {
    if (agentPickerPane.get() === pane) closeAgentPicker();
    if (menu) answerDialog(pane, menu.dialog.id, { kind: "cancelled" });
  };
  const togglePicker = (command: "/permissions", title: string) => {
    const click = pickerClick(menu?.dialog.title ?? null, title);
    closeMenus();
    if (click.open) openPicker(pane, command);
  };
  const toggleAgent = () => {
    const open = floating === "agent";
    closeMenus();
    if (!open) openAgentPicker(pane);
  };

  // Follow the reply while the reader sits at the bottom; reading further up pauses it. A session that opens starts the transcript over and shows its end.
  const firstItem = useRef(chat.items[0]);
  // Find searches again only when the shown rows change, not on each chunk of a streaming reply.
  const shownItems = useRef(chat.items);
  if (!sameItems(shownItems.current, chat.items)) shownItems.current = chat.items;
  const subagentRows = chat.capabilities?.includes("subagents") ? chat.subagents : noSubagents;
  const queuedRows = useMemo(() => (chat.held === null ? chat.queued : [chat.held]), [chat.held, chat.queued]);
  useLayoutEffect(() => {
    const el = transcript.current;
    if (chat.items[0] !== firstItem.current) pinned.current = true;
    firstItem.current = chat.items[0];
    if (el && pinned.current) el.scrollTop = el.scrollHeight;
  }, [shownItems.current, chat.running, queuedRows, chat.steerImages, subagentRows]);

  const jumpToEnd = () => {
    const el = transcript.current;
    if (!el) return;
    pinned.current = true;
    const smooth = !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    el.scrollTo({ top: el.scrollHeight, behavior: smooth ? "smooth" : "auto" });
  };

  const send = () => {
    if (!canSubmit) return;
    setDraft(pane, "");
    sendMessage(pane, draft.trim());
    composer.current?.focus();
  };

  const pickSlash = (command: SlashCommand) => {
    setDraft(pane, completion(command));
    composer.current?.focus();
  };

  const onComposerKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    const action = composerKey({ key: e.key, shift: e.shiftKey, composing: e.nativeEvent.isComposing }, draft, slashRows, slashAt);
    if (!action) return;
    e.preventDefault();
    if (action.kind === "step") setSlashActive(action.to);
    else if (action.kind === "complete") pickSlash(action.command);
    else send();
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
        // A click outside a control lands in the composer, unless the reader selected text. The agent menu moves the focus itself: a pick can open another tab.
        if ((e.target as Element).closest("button, a, input, textarea, .chat-agent-menu")) return;
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
          {chat.items.length === 0 && chat.agent === "starting" && chat.capabilities === null ? (
            <span className="spinner chat-starting" role="status" aria-label="Starting the agent" />
          ) : chat.items.length === 0 && chat.agent === "idle" ? (
            <div className="chat-empty">
              <h1 className="chat-empty-title">No chat yet</h1>
              <p className="chat-empty-text">{blocked ? "Run the command, then switch back. Chord checks again." : "Start an agent in this worktree."}</p>
              <Start worktree={worktree} pane={pane} blocking={blocking} />
            </div>
          ) : (
            <Messages pane={pane} worktree={worktree} running={chat.running} items={shownItems.current} queued={queuedRows} steerImages={chat.steerImages} subagents={subagentRows} subagentsAt={chat.subagentsAt} historyLength={chat.historyLength} />
          )}
        </div>
        <button type="button" className="chat-jump" data-shown={away || undefined} title="Jump to Latest" aria-label="Jump to Latest" onClick={jumpToEnd}>
          <ArrowDownIcon />
        </button>
        {find && <TranscriptFind pane={pane} find={find} transcript={transcript} items={shownItems.current} />}
        {chat.running && <Working since={chat.since ?? Date.now()} compacting={chat.compacting} />}
      </div>
      {viewing && <Lightbox images={viewing.images} index={viewing.index} onStep={(by) => stepLightbox(pane, by)} onClose={() => closeLightbox(pane)} />}
      {dropping && (
        <div className="chat-drop">
          <div className="chat-drop-content">
            <span className="chat-drop-icon">
              <ImageIcon size={24} />
            </span>
            <span className="chat-drop-title">Drop images to attach</span>
            <span className="chat-drop-text">They go with your next message in this chat.</span>
          </div>
        </div>
      )}
      {/* The rows say what a failed start's notice says. */}
      {chat.notice && !blocked && <p className="chat-notice">{chat.notice}</p>}
      {chat.agent === "idle" && chat.items.length > 0 && (
        <div className="chat-restart" data-blocked={blocked || undefined}>
          <span className="chat-hint">The agent stopped.</span>
          <Start worktree={worktree} pane={pane} blocking={blocking} />
        </div>
      )}
      {chat.dialog && !menu && <DialogPanel key={chat.dialog.id} pane={pane} dialog={chat.dialog} />}
      {composerShown && (
        <div className="chat-composer-anchor">
          {menu && <DialogMenu key={menu.dialog.id} pane={pane} dialog={menu.dialog} options={menu.ask.options} current={menu.ask.current ?? null} filter={menu.ask.filter} icon={menu.dialog.title === "Permissions" ? (mode) => <PermissionIcon mode={mode} /> : undefined} />}
          {floating === "agent" && harness && <AgentMenu worktree={worktree} pane={pane} harness={harness} chat={chat} trigger={agentButton} />}
          {floating === "slash" && <SlashMenu commands={slashRows} query={slashQuery(draft) ?? ""} active={slashAt} onActive={setSlashActive} onPick={pickSlash} onDismiss={() => closeSlash(pane, draft)} />}
          <div className="chat-composer" data-path-drop={pathOver || undefined}>
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
              {chat.permissions && (
                <button
                  type="button"
                  className="chat-picker chat-permissions"
                  title="Permissions"
                  disabled={!ready || chat.loading}
                  data-menu-toggle
                  data-asks={chat.permissions.asks || undefined}
                  aria-expanded={menu?.dialog.title === "Permissions"}
                  onClick={() => togglePicker("/permissions", "Permissions")}
                >
                  <span className="chat-permissions-icon">
                    <PermissionIcon mode={chat.permissions.mode} />
                  </span>
                  {chat.permissions.mode}
                </button>
              )}
              {context && (
                <span className="chat-context" data-level={context.level} title={context.title} aria-label={context.title}>
                  <svg className="chat-context-ring" viewBox="0 0 14 14" aria-hidden="true">
                    <circle cx="7" cy="7" r="5.5" pathLength="100" />
                    <circle cx="7" cy="7" r="5.5" pathLength="100" style={{ strokeDasharray: `${context.percent ?? 0} 100` }} />
                  </svg>
                  <span className="chat-context-percent">{context.percent === null ? "?" : `${context.estimated ? "~" : ""}${context.percent}%`}</span>
                </span>
              )}
              {labels && (
                <div className="chat-pickers">
                  <button
                    ref={agentButton}
                    type="button"
                    className="chat-picker chat-agent"
                    title={harness ? `${harnessMarks[harness]?.title ?? harness}, Model and Thinking` : "Model and Thinking"}
                    aria-haspopup="dialog"
                    aria-expanded={floating === "agent"}
                    disabled={!ready || chat.loading}
                    data-menu-toggle
                    onClick={toggleAgent}
                  >
                    {AgentLogo && (
                      <span className="chat-agent-logo">
                        <AgentLogo />
                      </span>
                    )}
                    <span className="chat-agent-model">{labels.name}</span>
                    {labels.thinking && <span className="chat-agent-level">{labels.thinking}</span>}
                  </button>
                </div>
              )}
              <button
                type="button"
                className="button-primary chat-send"
                title={steers ? "Steer" : "Send"}
                aria-label={steers ? "Steer" : "Send"}
                disabled={!canSubmit}
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
