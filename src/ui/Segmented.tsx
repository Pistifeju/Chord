import { useLayoutEffect, useRef, useState, type ReactNode } from "react";

export const onOff = [
  { value: true, label: "On" },
  { value: false, label: "Off" },
];

type Thumb = { x: number; width: number; jump: boolean };

export function Segmented<T extends string | boolean>({ label, options, value, onChange, sliding = false }: { label: string; options: { value: T; label: ReactNode; title?: string; disabled?: boolean }[]; value: T | null; onChange: (value: T) => void; sliding?: boolean }) {
  const root = useRef<HTMLDivElement>(null);
  const [thumb, setThumb] = useState<Thumb | null>(null);
  const set = options.map((option) => String(option.value)).join("\n");
  const shownSet = useRef(set);
  useLayoutEffect(() => {
    if (!sliding) return;
    const on = root.current?.querySelector<HTMLElement>('[aria-checked="true"]');
    const jump = shownSet.current !== set;
    shownSet.current = set;
    setThumb(on ? { x: on.offsetLeft, width: on.offsetWidth, jump } : null);
  }, [sliding, set, value]);
  return (
    <div className="segmented" role="radiogroup" aria-label={label} ref={root} data-sliding={sliding || undefined}>
      {sliding && thumb && <span className="segmented-thumb" aria-hidden="true" data-jump={thumb.jump || undefined} style={{ width: thumb.width, transform: `translateX(${thumb.x}px)` }} />}
      {options.map((option) => (
        <button key={String(option.value)} type="button" role="radio" title={option.title} disabled={option.disabled} aria-checked={value === option.value} onClick={() => onChange(option.value)}>
          <span className="segmented-label">{option.label}</span>
        </button>
      ))}
    </div>
  );
}
