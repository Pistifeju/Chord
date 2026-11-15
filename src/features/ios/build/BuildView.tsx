// The Build view in the bottom panel: the build's errors above its log.
import { useMemo } from "react";
import type { BuildIssue_Deserialize } from "../../../ipc/bindings";
import { WarningIcon } from "../../../ui/icons";
import { requestLine } from "../../editor/reveal";
import { showFile } from "../../editor/showing";
import { buildFinds, closeBuildFind, setBuildFindQuery } from "./find";
import { LogView } from "./LogView";
import { logLines } from "./log";
import { pathInWorktree } from "./run";
import { buildLogs, builds } from "./store";

function IssueRow({ worktree, issue }: { worktree: string; issue: BuildIssue_Deserialize }) {
  const name = issue.file?.split("/").pop();
  const place = name && (issue.line ? `${name}:${issue.line}` : name);
  const path = issue.file ? pathInWorktree(worktree, issue.file) : null;
  const content = (
    <>
      <span className="build-issue-icon">
        <WarningIcon />
      </span>
      {place && <span className="build-issue-place">{place}</span>}
      <span className="build-issue-message" title={issue.message}>
        {issue.message}
      </span>
    </>
  );
  if (path === null) return <li className="build-issue">{content}</li>;
  return (
    <li>
      <button
        type="button"
        className="build-issue"
        onClick={() => {
          if (issue.line) requestLine(worktree, path, issue.line);
          showFile(worktree, path);
        }}
      >
        {content}
      </button>
    </li>
  );
}

export function BuildView({ worktree }: { worktree: string }) {
  const build = builds.usePart((all) => all[worktree]);
  const log = buildLogs.usePart((all) => all[worktree]);
  const find = buildFinds.usePart((all) => all[worktree]);
  const lines = useMemo(() => (log ? logLines(log) : []), [log]);
  if (!build) return <div className="log-part log-empty">No build yet</div>;
  // A failed build shows its errors first.
  const failed = build.phase === "failed";
  return (
    <LogView
      view="build"
      label="Build log"
      lines={lines}
      restartKey={build.startedAt}
      head={
        failed && build.issues.length > 0 ? (
          <ul className="build-issues">
            {build.issues.map((issue, n) => (
              <IssueRow key={n} worktree={worktree} issue={issue} />
            ))}
            {build.more > 0 && <li className="build-more">and {build.more} more</li>}
          </ul>
        ) : undefined
      }
      atStart={failed}
      find={find}
      findLabel="Find in Build Log"
      onFindQuery={(query) => setBuildFindQuery(worktree, query)}
      onFindClose={() => closeBuildFind(worktree)}
    />
  );
}
