import { useEffect } from "react";
import { CloseIcon, WarningIcon } from "./icons";

export function WarningToast({
  text,
  notice = false,
  onDismiss,
}: {
  text: string;
  notice?: boolean;
  onDismiss: () => void;
}) {
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(onDismiss, 4000);
    return () => clearTimeout(timer);
  }, [text, notice, onDismiss]);
  return (
    <div className="toast" role={notice ? "status" : "alert"}>
      {!notice && (
        <span className="toast-warning">
          <WarningIcon />
        </span>
      )}
      <span className="toast-text" data-selectable>
        {text}
      </span>
      <button type="button" className="icon-button toast-close" title="Dismiss" aria-label="Dismiss" onClick={onDismiss}>
        <CloseIcon />
      </button>
    </div>
  );
}
