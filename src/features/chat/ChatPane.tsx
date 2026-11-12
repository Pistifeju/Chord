// The chat of one pane: transcript, composer, and a Start button while no agent runs.
import { memo, useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { ArrowUpIcon, ChevronIcon } from "../../ui/icons";
import { chatOf, chats, drafts, sendMessage, setDraft, startChat } from "./store";
import { shownSteps, stepsSummary, type Item, type Message, type StepGroup } from "./transcript";
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
      <span className="spinner-inline" />
      <span className="chat-working-verb">{VERBS[since % VERBS.length]}…</span>
      <span>{s < 60 ? `${s}s` : `${Math.floor(s / 60)}m ${s % 60}s`}</span>
      <span>esc stops</span>
    </div>
  );
}

const MessageRow = memo(function MessageRow({ message, fresh }: { message: Message; fresh: boolean }) {
  return (
    <div className="chat-message" data-role={message.role} data-fresh={fresh || undefined}>
      {message.text}
    </div>
  );
});

const StepsRow = memo(function StepsRow({ group, running, fresh }: { group: StepGroup; running: boolean; fresh: boolean }) {
  const [expanded, setExpanded] = useState(false);
  const shown = shownSteps(group.steps, running, expanded);
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
    </div>
  );
});

const Messages = memo(function Messages({ items, running }: { items: Item[]; running: boolean }) {
  // Only items that arrive while the pane is open animate in; reopening a long chat stays still.
  const loaded = useRef(items.length);
  return (
    <div className="chat-messages" data-selectable>
      {items.map((item, i) =>
        item.role === "steps" ? (
          <StepsRow key={i} group={item} running={running && i === items.length - 1} fresh={i >= loaded.current} />
        ) : item.streaming ? null : (
          <MessageRow key={i} message={item} fresh={i >= loaded.current} />
        ),
      )}
    </div>
  );
});

export const ChatPane = memo(function ChatPane({ worktree, pane, focused }: { worktree: string; pane: string; focused: boolean }) {
  const chat = chats.usePart((all) => chatOf(all, pane));
  const draft = drafts.usePart((all) => all[pane] ?? "");
  const composer = useRef<HTMLTextAreaElement>(null);
  const transcript = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);
  const ready = chat.agent === "ready";
  const hasChat = chat.capabilities === null || chat.capabilities.includes("chat");

  useEffect(() => {
    if (ready && focused) composer.current?.focus();
  }, [ready, focused]);

  // Follow the reply while the reader sits at the bottom; reading further up pauses it.
  useLayoutEffect(() => {
    const el = transcript.current;
    if (el && pinned.current) el.scrollTop = el.scrollHeight;
  }, [chat.items]);

  const send = () => {
    const text = draft.trim();
    if (!text) return;
    setDraft(pane, "");
    pinned.current = true;
    sendMessage(pane, text);
    composer.current?.focus();
  };

  const onComposerKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      send();
    }
  };

  if (!hasChat) return null;

  return (
    <div
      className="chat"
      onClick={(e) => {
        // A click outside a control lands in the composer, unless the reader selected text.
        if ((e.target as Element).closest("button, a, input, textarea")) return;
        if (window.getSelection()?.isCollapsed !== false) composer.current?.focus();
      }}
    >
      <div
        className="chat-transcript"
        ref={transcript}
        onScroll={(e) => {
          const el = e.currentTarget;
          pinned.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
        }}
      >
        {chat.items.length === 0 && chat.agent === "idle" ? (
          <div className="chat-empty">
            <h1 className="chat-empty-title">No chat yet</h1>
            <p className="chat-empty-text">Start an agent in this worktree.</p>
            <StartButton worktree={worktree} pane={pane} />
          </div>
        ) : (
          <Messages items={chat.items} running={chat.running} />
        )}
      </div>
      {chat.notice && <p className="chat-notice">{chat.notice}</p>}
      {chat.running && <Working since={chat.since ?? Date.now()} />}
      {chat.agent === "idle" && chat.items.length > 0 && (
        <div className="chat-restart">
          <span className="chat-hint">The agent stopped.</span>
          <StartButton worktree={worktree} pane={pane} />
        </div>
      )}
      {chat.agent !== "idle" && (
        <div className="chat-composer">
          <textarea
            ref={composer}
            className="chat-input"
            rows={2}
            disabled={!ready}
            value={draft}
            placeholder={chat.running ? "Steer the agent" : "Message the agent"}
            onChange={(e) => setDraft(pane, e.target.value)}
            onKeyDown={onComposerKey}
          />
          <div className="chat-composer-bar">
            <button
              type="button"
              className="button-primary chat-send"
              title={chat.running ? "Steer" : "Send"}
              aria-label={chat.running ? "Steer" : "Send"}
              disabled={!ready || !draft.trim()}
              onClick={send}
            >
              <ArrowUpIcon />
            </button>
          </div>
        </div>
      )}
    </div>
  );
});
