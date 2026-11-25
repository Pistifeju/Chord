import { useId, type ReactNode } from "react";

type Props = {
  title: string;
  description: ReactNode;
  detail?: string;
  confirmLabel: string;
  onCancel: () => void;
  onConfirm: () => void;
};

export function ConfirmSheet({ title, description, detail, confirmLabel, onCancel, onConfirm }: Props) {
  const descriptionId = useId();
  return (
    <div className="sheet-backdrop" onMouseDown={onCancel}>
      <div
        className="sheet"
        role="alertdialog"
        aria-modal="true"
        aria-label={title.replace(/\?$/, "")}
        aria-describedby={descriptionId}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="sheet-text">
          <h2 className="sheet-title">{title}</h2>
          <p className="sheet-description" id={descriptionId}>
            {description}
          </p>
          {detail && (
            <p className="sheet-meta" data-selectable>
              {detail}
            </p>
          )}
        </div>
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
