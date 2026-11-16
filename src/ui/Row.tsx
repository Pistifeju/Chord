// A settings row: a title with an optional description on the left, the control on the right; a nested row sits indented under the row above.
import type { ReactNode } from "react";

export function Row({ title, description, nested, children }: { title: ReactNode; description?: ReactNode; nested?: boolean; children: ReactNode }) {
  return (
    <div className="settings-row" data-nested={nested}>
      <div className="settings-row-text">
        <span className="settings-label">{title}</span>
        {description}
      </div>
      {children}
    </div>
  );
}
