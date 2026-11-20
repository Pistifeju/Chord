// The agent menu floating above the composer: the agents the user can run in a sliding bar, the shown agent's models, and a thinking bar whose thumb follows the model under the pointer. A pick switches the model, or opens a new chat tab on the other agent.
import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type RefObject } from "react";
import type { ModelOption } from "../../ipc/bindings";
import { CheckIcon } from "../../ui/icons";
import { steppedIndex } from "../../ui/listStep";
import { useOutsidePointer } from "../../ui/outsidePointer";
import { Segmented } from "../../ui/Segmented";
import { harnessMarks } from "../harnesses/logos";
import { checkHarnesses, harnessChecks } from "../harnesses/store";
import { closeAgentPicker, levelSlots, pickerAgents, shownLevel } from "./agentPicker";
import { typedIndex, TYPING_PAUSE_MS } from "./dialog";
import { levelLabel, providerLabel } from "./model";
import { newSessionOn } from "../panes/store";
import { focusComposer, modelCatalogs, pickModel } from "./store";
import type { Chat } from "./transcript";

const noModels: ModelOption[] = [];

const titleOf = (harness: string) => harnessMarks[harness]?.title ?? harness;

type Props = { worktree: string; pane: string; harness: string; chat: Chat; trigger: RefObject<HTMLButtonElement | null> };

export function AgentMenu({ worktree, pane, harness, chat, trigger }: Props) {
  const checks = harnessChecks.usePart((s) => s.checks);
  const agents = pickerAgents(checks, harness);
  const [shown, setShown] = useState(harness);
  const models = modelCatalogs.usePart((all) => all[shown]) ?? noModels;
  const own = shown === harness;
  const [active, setActive] = useState(() => Math.max(0, models.findIndex((m) => m.id === chat.model?.id)));
  const [hovered, setHovered] = useState<number | null>(null);
  const [chosen, setChosen] = useState<Record<string, string>>({});
  const card = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const typed = useRef({ text: "", at: 0 });
  useOutsidePointer(card, closeAgentPicker);

  const target = models[hovered ?? active] ?? null;
  const slots = levelSlots(models);
  const level = target ? shownLevel(target, chat.model, chosen) : null;

  useEffect(() => {
    if (!harnessChecks.get().checks) void checkHarnesses();
  }, []);

  // The card grows out of the button: its CSS entrance scales from the button's center on the card's bottom edge.
  useLayoutEffect(() => {
    const el = card.current;
    if (!el) return;
    el.focus({ preventScroll: true });
    const button = trigger.current?.getBoundingClientRect();
    if (button) el.style.transformOrigin = `${button.left + button.width / 2 - el.getBoundingClientRect().left}px 100%`;
  }, [trigger]);

  useEffect(() => {
    list.current?.querySelector("[data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active, shown]);

  const showAgent = (name: string) => {
    if (name === shown) return;
    setShown(name);
    const list = modelCatalogs.get()[name] ?? noModels;
    setActive(name === harness ? Math.max(0, list.findIndex((m) => m.id === chat.model?.id)) : 0);
    setHovered(null);
  };

  // The pane's own agent takes the level at once; for another agent it waits for the model pick that starts it.
  const setLevel = (value: string) => {
    if (!target) return;
    setChosen((was) => ({ ...was, [target.id]: value }));
    if (own) pickModel(pane, target.id, value);
  };

  const pick = (at: number | null) => {
    const option = at === null ? null : models[at];
    const picked = option ? shownLevel(option, chat.model, chosen) : null;
    if (own) {
      if (option && !(option.id === chat.model?.id && picked === chat.model.thinking)) pickModel(pane, option.id, picked);
      closeAgentPicker();
      focusComposer(pane);
      return;
    }
    closeAgentPicker();
    newSessionOn(worktree, shown, option ? { id: option.id, level: picked } : null);
  };

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Tab") {
      e.preventDefault();
      const at = agents.indexOf(shown);
      showAgent(agents[(at + (e.shiftKey ? -1 : 1) + agents.length) % agents.length]);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setHovered(null);
      setActive((at) => Math.max(0, steppedIndex(at, e.key === "ArrowDown" ? 1 : -1, models.length)));
    } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      const levels = models[active]?.levels ?? [];
      if (levels.length === 0) return;
      e.preventDefault();
      setHovered(null);
      const at = levels.indexOf(shownLevel(models[active], chat.model, chosen) ?? "");
      const by = e.key === "ArrowRight" ? 1 : -1;
      const next = at === -1 ? (by === 1 ? 0 : levels.length - 1) : Math.min(levels.length - 1, Math.max(0, at + by));
      if (next !== at) {
        setChosen((was) => ({ ...was, [models[active].id]: levels[next] }));
        if (own) pickModel(pane, models[active].id, levels[next]);
      }
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing) {
      e.preventDefault();
      pick(models.length > 0 ? active : null);
    } else if (e.key.length === 1 && !e.metaKey && !e.ctrlKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      const text = (e.timeStamp - typed.current.at < TYPING_PAUSE_MS ? typed.current.text : "") + e.key;
      typed.current = { text, at: e.timeStamp };
      const at = typedIndex(
        models.map((m) => m.name),
        text,
      );
      if (at !== -1) {
        setHovered(null);
        setActive(at);
      }
    }
  };

  const title = titleOf(shown);

  return (
    <div className="chat-slash-menu chat-agent-menu" role="dialog" aria-label="Agent and Model" tabIndex={-1} ref={card} onKeyDown={onKey}>
      <div className="chat-agent-body">
        {agents.length > 1 && (
          <div className="chat-agent-tabs">
            <Segmented
              sliding
              label="Agent"
              value={shown}
              onChange={showAgent}
              options={agents.map((name) => {
                const Logo = harnessMarks[name]?.Logo;
                return {
                  value: name,
                  title: name === harness ? "Runs in this pane" : undefined,
                  label: (
                    <>
                      {Logo && <Logo />}
                      {titleOf(name)}
                    </>
                  ),
                };
              })}
            />
          </div>
        )}
        <div className="chat-agent-list" role="listbox" aria-label={`${title} Models`} ref={list} key={shown} onPointerLeave={() => setHovered(null)}>
          {models.length === 0 ? (
            <div role="option" aria-selected data-active className="chat-dialog-option" onClick={() => pick(null)}>
              <span className="chat-dialog-name">Default Model</span>
            </div>
          ) : (
            models.map((model, at) => {
              const isCurrent = own && model.id === chat.model?.id;
              return (
                <div
                  key={model.id}
                  role="option"
                  aria-selected={at === active}
                  data-active={(hovered ?? active) === at || undefined}
                  className="chat-dialog-option"
                  onPointerMove={() => hovered !== at && setHovered(at)}
                  onClick={() => pick(at)}
                >
                  <span className="chat-dialog-name">{model.name}</span>
                  <span className="chat-dialog-meta">
                    {model.provider && providerLabel(model.provider)}
                    <span className="chat-dialog-current" role={isCurrent ? "img" : undefined} aria-label={isCurrent ? "Current" : undefined}>
                      {isCurrent && <CheckIcon />}
                    </span>
                  </span>
                </div>
              );
            })
          )}
        </div>
        {models.length > 0 && (
          <div className="chat-agent-thinking">
            <span className="chat-agent-note">Thinking</span>
            {/* The levels a highlighted model lacks stay in place, disabled, so no segment shifts as the highlight moves. */}
            {slots.length > 0 ? (
              <Segmented sliding label="Thinking" value={level} onChange={setLevel} options={slots.map((value) => ({ value, label: levelLabel(value), disabled: !target?.levels.includes(value) }))} />
            ) : (
              <span className="chat-agent-note chat-agent-none">No thinking levels</span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
