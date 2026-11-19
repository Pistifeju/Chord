// The sidebar's Tests view in a worktree with an Xcode project: its header with the Run All Tests button, disabled until Chord runs tests, over the line that stands in for the test list.
import { TestIcon } from "../../../ui/icons";

export function TestsView() {
  return (
    <div className="files">
      <div className="files-header">
        <span className="files-title">Tests</span>
        <button type="button" className="icon-button" title="Run All Tests" aria-label="Run All Tests" disabled>
          <TestIcon />
        </button>
      </div>
      <div className="files-empty">Chord does not run tests yet.</div>
    </div>
  );
}
