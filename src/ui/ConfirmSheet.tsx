// The confirm sheet: a title, one description line, an optional detail, then Cancel and the confirming action.
import type { ReactNode } from "react";

type Props = {
  title: string;
  description: ReactNode;
  detail?: string;
  confirmLabel: string;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ConfirmSheet({ title, description, detail, confirmLabel, onCancel, onConfirm }: Props) {
  return (
    <div className="sheet-scrim" onMouseDown={onCancel}>
      <div className="sheet" role="alertdialog" aria-label={title.replace(/\?$/, "")} onMouseDown={(e) => e.stopPropagation()}>
        <h2 className="sheet-title">{title}</h2>
        <p className="sheet-description">{description}</p>
        {detail && <p className="sheet-meta" data-selectable>{detail}</p>}
        <footer className="sheet-actions">
          <button type="button" className="button-secondary" onClick={onCancel}>
            Cancel
          </button>
          <button type="button" className="button-destructive" autoFocus onClick={onConfirm}>
            {confirmLabel}
          </button>
        </footer>
      </div>
    </div>
  );
}
