// An icon button that copies a text and shows a check for 1.5s after it copied.
import { useEffect, useState } from "react";
import { CheckIcon, CopyIcon } from "./icons";

export function CopyButton({ text, label, className }: { text: string; label: string; className: string }) {
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(timer);
  }, [copied]);
  return (
    <button type="button" className={`icon-button ${className}`} title={label} aria-label={label} onClick={() => navigator.clipboard.writeText(text).then(() => setCopied(true), console.error)}>
      {copied ? <CheckIcon /> : <CopyIcon />}
    </button>
  );
}
