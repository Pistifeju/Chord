// A segmented radio control: one button per option, the chosen one checked; a null value checks none.
export const onOff = [
  { value: true, label: "On" },
  { value: false, label: "Off" },
];

export function Segmented<T extends string | boolean>({ label, options, value, onChange }: { label: string; options: { value: T; label: string }[]; value: T | null; onChange: (value: T) => void }) {
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <button key={String(option.value)} type="button" role="radio" aria-checked={value === option.value} onClick={() => onChange(option.value)}>
          {option.label}
        </button>
      ))}
    </div>
  );
}
