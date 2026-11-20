// A segmented radio control: one button per option, the chosen one checked; a null value checks none. A sliding one moves a single thumb to the checked option.
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
  // A new set of options lays out new buttons, so the thumb jumps there instead of sliding from a button that is gone.
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
          {option.label}
        </button>
      ))}
    </div>
  );
}
