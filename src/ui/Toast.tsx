// Top-center toast in the title bar row; closes on click or after 4 seconds.
import { useEffect } from "react";
import { WarningIcon } from "./icons";

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
    const timer = setTimeout(onDismiss, 4000);
    return () => clearTimeout(timer);
  }, [text, onDismiss]);
  return (
    <button type="button" className="toast" role="alert" onClick={onDismiss}>
      {!notice && (
        <span className="toast-warning">
          <WarningIcon />
        </span>
      )}
      {text}
    </button>
  );
}
