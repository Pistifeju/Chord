# Inputs

- **Field** (`.input`): 30px tall, `sheet` fill, 1px `line-strong` border, `ink-muted` placeholder. The label sits above at 12px/500; the hint sits below at 12px muted and reserves its line so the sheet keeps its height. A hint that holds an error (a branch git refuses, a failed create) takes `danger`.
- **Focus:** no visual change. The caret shows where you type. No outline, no blue border, no halo.
- **Segmented control** (`.segmented`): a `raised` track with 2px padding. The selected segment is a `control-selected` chip with the control shadow.
- **Shortcut recorder:** a secondary button that shows the shortcut, or "None" in muted ink. While recording it gets a `canvas` fill, a 1px `accent-ink` inset border and a 3px `accent-wash` halo, and reads "Press keys".
