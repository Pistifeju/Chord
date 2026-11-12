// The chat of one worktree: transcript, composer, and a Start button while no agent runs.
import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { ArrowUpIcon } from "../../ui/icons";
import { chatOf, chats, sendMessage, startChat } from "./store";
import "./chat.css";

function StartButton({ worktree }: { worktree: string }) {
  return (
    <button type="button" className="button-primary" onClick={() => startChat(worktree)}>
      Start
    </button>
  );
}

export function ChatPane({ worktree }: { worktree: string }) {
  const chat = chatOf(chats.use(), worktree);
  const [draft, setDraft] = useState("");
  const composer = useRef<HTMLTextAreaElement>(null);
  const transcript = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);
  const ready = chat.agent === "ready";
  const hasChat = chat.capabilities === null || chat.capabilities.includes("chat");

  useEffect(() => {
    if (ready) composer.current?.focus();
  }, [ready]);

  // Follow the reply while the reader sits at the bottom; reading further up pauses it.
  useLayoutEffect(() => {
    const el = transcript.current;
    if (el && pinned.current) el.scrollTop = el.scrollHeight;
  }, [chat.messages]);

  const send = () => {
    const text = draft.trim();
    if (!text) return;
    setDraft("");
    pinned.current = true;
    sendMessage(worktree, text);
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
    <div className="chat">
      <div
        className="chat-transcript"
        ref={transcript}
        onScroll={(e) => {
          const el = e.currentTarget;
          pinned.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
        }}
      >
        {chat.messages.length === 0 && chat.agent === "idle" ? (
          <div className="chat-empty">
            <h1 className="chat-empty-title">No chat yet</h1>
            <p className="chat-empty-text">Start an agent in this worktree.</p>
            <StartButton worktree={worktree} />
          </div>
        ) : (
          <div className="chat-messages" data-selectable>
            {chat.messages.map((message, i) => (
              <div key={i} className="chat-message" data-role={message.role}>
                {message.text}
              </div>
            ))}
            {chat.running && <div className="chat-working">Working</div>}
          </div>
        )}
      </div>
      {chat.notice && <p className="chat-notice">{chat.notice}</p>}
      {chat.agent === "idle" && chat.messages.length > 0 && (
        <div className="chat-restart">
          <span className="chat-hint">The agent stopped.</span>
          <StartButton worktree={worktree} />
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
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={onComposerKey}
          />
          <div className="chat-composer-bar">
            <span className="chat-hint">{chat.running ? "Esc to stop" : ""}</span>
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
}
