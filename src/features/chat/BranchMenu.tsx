// The branch menu floating above the composer: local branches newest first, switch, and New Branch.
import { useEffect, useRef, useState } from "react";
import { commands, type Branch } from "../../ipc/bindings";
import { useOutsidePointer } from "../../ui/outsidePointer";
import { refreshWorktrees } from "../worktrees/store";
import { Select } from "./DialogPanel";

export function BranchMenu({ worktree, project, current, onClose }: { worktree: string; project: string; current: string | null; onClose: () => void }) {
  const menu = useRef<HTMLDivElement>(null);
  const [branches, setBranches] = useState<Branch[] | null>(null);
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  useOutsidePointer(menu, onClose);

  useEffect(() => {
    commands.listBranches(worktree).then(setBranches, (error) => setProblem(String(error)));
  }, [worktree]);

  const change = async (switching: Promise<null>) => {
    try {
      await switching;
      await refreshWorktrees(project);
      onClose();
    } catch (error) {
      setProblem(String(error));
    }
  };

  return (
    <div
      className="chat-slash-menu chat-dialog-menu"
      role="dialog"
      aria-label="Branches"
      ref={menu}
      onKeyDown={(e) => {
        if (e.key === "Escape") onClose();
      }}
    >
      {naming ? (
        <input
          autoFocus
          className="input"
          value={name}
          placeholder="Branch name"
          aria-label="New Branch Name"
          onChange={(e) => {
            setName(e.target.value);
            setProblem(null);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.nativeEvent.isComposing && name.trim()) change(commands.createBranch(worktree, name.trim()));
          }}
        />
      ) : (
        <>
          {branches && (
            <Select
              options={branches.map((b) => b.name)}
              current={current}
              unavailable={branches.filter((b) => b.held_elsewhere).map((b) => b.name)}
              unavailableTitle="Checked out in another worktree"
              answer={(answer) => answer.kind === "value" && answer.value !== current && change(commands.switchBranch(worktree, answer.value))}
            />
          )}
          <button type="button" className="chat-branch-new" onClick={() => setNaming(true)}>
            New Branch
          </button>
        </>
      )}
      {problem && <p className="chat-branch-problem">{problem}</p>}
    </div>
  );
}
