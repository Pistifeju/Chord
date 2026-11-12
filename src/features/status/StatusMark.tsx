// The mark of an agent's status, the same in every place: two orbiting dots while it runs, otherwise a dot, in the status color.
import type { Status } from "../../ipc/bindings";
import { statusLabel } from "./status";

export function StatusMark({ kind }: { kind: Status["kind"] }) {
  // The running mark often stands without its word, so it names itself.
  if (kind === "running") return <span className="status-orbit" data-status={kind} role="img" aria-label={statusLabel({ kind })} />;
  return <span className="status-dot" data-status={kind} />;
}
