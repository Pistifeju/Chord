# Usage panel

- A panel centered over the window, which blurs behind it as behind a sheet: `canvas`, Card radius, sheet shadow, 960 × 760px at most and 28px clear of the window edges, like Settings.
  It rises in like a sheet.
  ⌘⇧U, Chord › Usage or a click on a usage group in the status bar opens it.
  Esc, the close icon button or a click outside it closes it; a sheet opened over it closes first, and a press that moves 4px first drags the window instead.
  The window behind it is inert.
  Opening it closes Settings, and opening Settings closes it.
- **Header:** 48px with a 1px `line` bottom rule, 28px left padding: "Usage" at Heading size, then a Segmented for the range ("24h", "7d", "30d", "90d", each with its tooltip such as "Past 7 days"), one for Cost or Tokens, a refresh icon button and a close icon button.
  The refresh icon turns once every 800ms while a read runs.
  The panel opens on Past 30 days and Cost.
- **List:** fills the width left of the inspector, scrolls, 12px padding before the scrollbar's room.
  One `role="listbox"` that takes the focus when the panel shows its rows and names the picked row with `aria-activedescendant`; ↑ ↓ Home End move the pick and scroll it into view, a click picks.
  Rows are 36px, 12px side padding, Control radius:
  - "All agents" first, 40px, 600, `ink`.
  - Each harness, 12px above it, ranked by the shown value (ties by tokens): its rank color as an 8px dot, its 16px logo, its title at 500 in `ink`, its share, its value.
  - Its models under it, 48px in, in `ink-secondary`: the name on one line, clipped, with a 2px `accent-ink` bar under it as wide as its share of the total; the share in 12px `ink-muted` in a 56px column; the value in a 96px tabular column.
  - Hover takes `raised` over 120ms with `--ease-out`.
    The picked row takes `selected` and `ink`, and its share steps up to `ink-secondary`.
    While the list holds the focus, the picked row carries a 2px inset `focus-ring`.
  - In Cost mode a row with no recorded cost (Codex, Claude Code) shows no share and no value.
- **Rank colors:** the harness with the largest share in the range takes `accent-ink`, the second `ink-secondary`, the third `ink-disabled`.
  The dot and the chart line use the same color.
- **Inspector:** 340px, `panel`, a 1px `line` left edge, 20px 18px 28px 20px padding plus the scrollbar's room, 20px between its parts, scrolls.
  For the picked row:
  - The title: the harness logo and the name at Heading size; a model name wraps and is selectable.
    It fades in and rises 4px over 200ms with `--ease-out` when the pick changes.
  - The total at 28px, 600, line height 1.15, -0.02em, tabular figures.
    Under it, 4px down, a meta line: "208 sessions · Past 30 days"; a model shows the range alone, since the session counts are per harness.
    Cost mode for a pick with no cost shows "No cost recorded" at 16px in `ink-muted`.
  - The chart, 280 × 140, an inline SVG: four horizontal grid lines in `line` with their values in 12px `ink-muted` at the left ("$40", "1.5B"), three date labels under the axis ("Jun 12"; hours such as "3 PM" for Past 24h), one 1.5px line per harness in its rank color, and an `accent-wash` fill under the first-ranked harness's line.
    It plots local days, or hours for Past 24h.
    Cost mode leaves out a harness with no cost.
    `role="img"` with a label that states the range and the total.
  - Tokens by type: an 8px bar of four segments 2px apart with 2px corners, each at least 2px wide: Input `ink-secondary`, Cache read `ink-muted`, Cache write `ink-disabled`, Output `line-strong`.
    Under it, 8px down, a 12px legend in `ink-secondary`: an 8px swatch, the name, the count in `ink-muted`.
    The bar's `aria-label` lists each type's share.
  - Five figures as rows 6px top and bottom, split by `line` rules: the name in `ink-muted` on the left, the value at 500 with tabular figures on the right.
    Processed tokens, Cached input (cache reads), Uncached input (input plus cache write), Output, Cache savings ("No cost recorded" with no pi rows).
  - "Busiest days" ("Busiest hours" for Past 24h) as a field label, 4px above three rows of the same figure style: "Tue, Jun 12" and its value, most first.
    Cost mode lists only days with a cost.
- **Numbers:** tokens compact ("6.92B", "123M"), cost in US dollars with cents, shares to one decimal, "<0.1%" above 0 and under 0.1%, all in the system's locale.
- **States:** "Loading" in 12px `ink-muted`, centered, until the first read of the app run ends; the read ahead below usually ends it before the panel opens.
  "No usage in this range" at Empty title size, centered, when the read has no rows.
  A later read keeps the shown rows while the refresh icon turns.
- **Reads:** on open, on a range change and on refresh.
  A read that a newer one overtook is dropped.
  The last read shows while a new one runs, cut to the shown range.
  2 s after launch the app reads 90 days in the background, which fills Rust's cache for every range (about 3 s once per app run), then the panel's opening range; later reads take about 50 ms.
  Rust runs one read at a time, so a read during another waits for it and then finds the cache full.
- **Scrollbars:** the list and the inspector keep the scrollbar's 10px reserved (`scrollbar-gutter: stable`), so their values stay put when it shows.
  The list's right padding is 2px and the inspector's 18px, so the content keeps 12px and 28px to the panel edge.
- **Reduce Motion:** the panel and the inspector title only fade, and the refresh icon turns at a third of its speed.
  The chart and the bars never animate.
