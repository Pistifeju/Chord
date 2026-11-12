// The slash menu floating above the composer card: the commands that match what the draft holds, the typed part marked.
import { useEffect, useRef } from "react";
import type { SlashCommand } from "../../ipc/bindings";
import { ChatIcon, ExtensionIcon, PromptIcon, SkillIcon } from "../../ui/icons";
import { commandSource, commandTitle, markedTitle } from "./slash";

const kindIcon = { skill: SkillIcon, prompt: PromptIcon, extension: ExtensionIcon, session: ChatIcon };

export function SlashMenu({ commands, query, active, onActive, onPick }: { commands: SlashCommand[]; query: string; active: number; onActive: (index: number) => void; onPick: (command: SlashCommand) => void }) {
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    list.current?.querySelector("[data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active]);
  return (
    <div className="chat-slash-menu" role="listbox" aria-label="Commands" ref={list}>
      {commands.map((command, i) => {
        const { before, typed, after } = markedTitle(commandTitle(command.name), query);
        const KindIcon = kindIcon[command.kind];
        return (
          <div
            key={command.name}
            role="option"
            aria-selected={i === active}
            data-active={i === active || undefined}
            className="chat-slash-option"
            // The composer keeps focus through a click.
            onMouseDown={(e) => e.preventDefault()}
            onMouseMove={() => i !== active && onActive(i)}
            onClick={() => onPick(command)}
          >
            <span className="chat-slash-icon">
              <KindIcon />
            </span>
            <span className="chat-slash-name">
              {before}
              <span className="chat-slash-typed">{typed}</span>
              {after}
            </span>
            <span className="chat-slash-description">{command.description}</span>
            <span className="chat-slash-source">{commandSource(command)}</span>
          </div>
        );
      })}
    </div>
  );
}
