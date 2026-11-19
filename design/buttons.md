# Buttons

- **Primary** (`.button-primary`): blue fill, white 13px/500 label, 28px tall. One per view. No hover change, per macOS; pressed steps to `accent-pressed`.
- **Disabled primary:** `raised` fill, `ink-disabled` label. A form keeps the button disabled until its input is valid.
- **Loading:** the label hides and a 14px spinner takes its place in the same button (`aria-busy="true"`).
- **Secondary** (`.button-secondary`): `sheet` fill with a 1px `line-strong` inner border and the control shadow. Hover lays `raised` over it, pressed `selected`, both over 120ms.
- **Destructive** (`.button-destructive`): a secondary button with a `danger` label.
- **Icon button** (`.icon-button`): 28px square, no fill at rest, `raised` on hover, `selected` when pressed, its icon pressing to 90% scale over 60ms (none under Reduce Motion); the text size steppers, a tab's close button and the Simulator view's buttons press their icon the same way. Each has a tooltip with its shortcut.
- **Icon button disabled:** `ink-disabled` glyph, no hover fill.
- **Focus:** a 2px `focus-ring` outline, 2px outside the control.
