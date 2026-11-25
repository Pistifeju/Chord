import type { Status } from "../../ipc/bindings";
import { statusLabel } from "./status";

export function StatusMark({ kind }: { kind: Status["kind"] }) {
  if (kind === "running") return <span className="status-orbit" data-status={kind} role="img" aria-label={statusLabel({ kind })} />;
  return <span className="status-dot" data-status={kind} />;
}
