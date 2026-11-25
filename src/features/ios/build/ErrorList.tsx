import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type RefObject } from "react";
import { createPortal } from "react-dom";
import type { BuildIssue_Deserialize } from "../../../ipc/bindings";
import { WarningIcon } from "../../../ui/icons";
import { steppedIndex } from "../../../ui/listStep";
import { useOutsidePointer } from "../../../ui/outsidePointer";
import { showFileAt } from "../../editor/showing";
import { fileName } from "../../files/paths";
import { closeErrorList } from "./errorListStore";
import { issueGroups, pathInWorktree, type Build } from "./run";
import { showBuildLog } from "./store";

export function ErrorList({ worktree, build, anchor }: { worktree: string; build: Build; anchor: RefObject<HTMLButtonElement | null> }) {
  const panel = useRef<HTMLDivElement>(null);
  const [spot, setSpot] = useState<{ top: number; right: number } | null>(null);
  const [active, setActive] = useState(0);
  const groups = issueGroups(build.issues);
  const issues = groups.flatMap((group) => group.issues);
  const pathOf = (issue: BuildIssue_Deserialize) => (issue.file ? pathInWorktree(worktree, issue.file) : null);

  const close = () => {
    closeErrorList();
    anchor.current?.focus();
  };
  const open = (issue: BuildIssue_Deserialize) => {
    const path = pathOf(issue);
    if (path === null) return;
    showFileAt(worktree, path, issue.line);
    closeErrorList();
  };

  useOutsidePointer(panel, closeErrorList);
  useLayoutEffect(() => {
    const box = anchor.current?.getBoundingClientRect();
    if (box) setSpot({ top: box.bottom + 6, right: window.innerWidth - box.right });
    panel.current?.focus();
    window.addEventListener("resize", closeErrorList);
    return () => window.removeEventListener("resize", closeErrorList);
  }, [anchor]);
  useEffect(() => {
    panel.current?.querySelector("[data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") setActive(steppedIndex(active, e.key === "ArrowDown" ? 1 : -1, issues.length));
    else if (e.key === "Enter" && issues[active]) open(issues[active]);
    else if (e.key === "Escape") close();
    else return;
    e.preventDefault();
    e.stopPropagation();
  };

  let row = -1;
  return createPortal(
    <div
      ref={panel}
      className="error-list"
      role="dialog"
      aria-label="Build Errors"
      tabIndex={-1}
      style={spot ?? { visibility: "hidden" }}
      onKeyDown={onKey}
    >
      <div className="error-list-rows" role="listbox" aria-label="Errors" aria-activedescendant={`error-list-${active}`}>
        {groups.map((group) => (
          <div key={group.file ?? ""} role="group" aria-label={group.file ? fileName(group.file) : "Build"}>
            <div className="error-list-group">
              <span>{group.file ? fileName(group.file) : "Build"}</span>
              <span className="error-list-count">{group.issues.length}</span>
            </div>
            {group.issues.map((issue) => {
              row += 1;
              const index = row;
              return (
                <div
                  key={index}
                  id={`error-list-${index}`}
                  role="option"
                  aria-selected={index === active}
                  aria-disabled={pathOf(issue) === null || undefined}
                  className="error-list-row"
                  data-active={index === active || undefined}
                  title={issue.message}
                  onMouseMove={() => index !== active && setActive(index)}
                  onClick={() => open(issue)}
                >
                  <span className="error-list-icon">
                    <WarningIcon />
                  </span>
                  <span className="error-list-message">{issue.message}</span>
                  {issue.line && <span className="error-list-line">{issue.line}</span>}
                </div>
              );
            })}
          </div>
        ))}
        {build.more > 0 && <p className="error-list-more">and {build.more} more in the build log</p>}
      </div>
      <div className="error-list-foot">
        <span className="error-list-hint">↑↓ to move · ↩ to open</span>
        <button
          type="button"
          className="error-list-log"
          onClick={() => {
            showBuildLog(worktree);
            closeErrorList();
          }}
        >
          Show Build Log
        </button>
      </div>
    </div>,
    document.body,
  );
}
