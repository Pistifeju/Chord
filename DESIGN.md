---
name: Chord
description: "A macOS workspace for coding agents: zinc neutrals, a near-white or pure black canvas, one blue accent, flat chrome. Light and dark share one set of roles."
colors:
  canvas: "#fcfcfc"
  panel: "#fafafa"
  raised: "#f4f4f5"
  selected: "#e4e4e7"
  sheet: "#ffffff"
  control-selected: "#ffffff"
  bubble: "#ebebed"
  line: "#e4e4e7"
  line-strong: "#d4d4d8"
  ink: "#18181b"
  ink-secondary: "#3f3f46"
  ink-muted: "#71717b"
  ink-disabled: "#a1a1aa"
  accent: "#1b4ed8"
  accent-pressed: "#1a44bd"
  accent-ink: "#1b4ed8"
  accent-wash: "rgba(27, 78, 216, 0.14)"
  on-accent: "#ffffff"
  focus-ring: "rgba(27, 78, 216, 0.6)"
  scrim: "rgba(0, 0, 0, 0.3)"
  bezel: "#18181b"
  danger: "#e7000b"
  success: "#007a55"
  status-running: "#155dfc"
  status-waiting: "#bb4d00"
  status-done: "#007a55"
  status-error: "#e7000b"
  git-changed: "#bb4d00"
  diff-added: "rgba(0, 122, 85, 0.1)"
  diff-removed: "rgba(231, 0, 11, 0.08)"
  dark-canvas: "#0f1011"
  dark-panel: "#18191a"
  dark-raised: "rgba(255, 255, 255, 0.05)"
  dark-selected: "rgba(255, 255, 255, 0.09)"
  dark-sheet: "#18191a"
  dark-control-selected: "rgba(255, 255, 255, 0.1)"
  dark-bubble: "#1e1f22"
  dark-line: "rgba(255, 255, 255, 0.08)"
  dark-line-strong: "rgba(255, 255, 255, 0.12)"
  dark-ink: "#f5f5f5"
  dark-ink-secondary: "#d4d4d4"
  dark-ink-muted: "#a1a1a1"
  dark-ink-disabled: "#6b6b6b"
  dark-accent: "#346bf1"
  dark-accent-pressed: "#2a5cd6"
  dark-accent-ink: "#6f9bff"
  dark-accent-wash: "rgba(52, 107, 241, 0.28)"
  dark-focus-ring: "rgba(111, 155, 255, 0.75)"
  dark-scrim: "rgba(0, 0, 0, 0.6)"
  dark-bezel: "#27272a"
  dark-danger: "#fb414a"
  dark-success: "#00bc7d"
  dark-status-running: "#2b7fff"
  dark-status-waiting: "#fe9a00"
  dark-status-done: "#00bc7d"
  dark-status-error: "#fb414a"
  dark-git-changed: "#fe9a00"
  dark-diff-added: "rgba(0, 188, 125, 0.14)"
  dark-diff-removed: "rgba(251, 65, 74, 0.14)"
typography:
  empty-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Inter Variable', 'Helvetica Neue', sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.45
  heading:
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.45
  body:
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontSize: "13px"
    fontWeight: 500
    lineHeight: 1.45
  meta:
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.45
rounded:
  check: "4px"
  inset: "6px"
  control: "8px"
  card: "16px"
spacing:
  xxs: "4px"
  xs: "8px"
  sm: "12px"
  md: "16px"
  lg: "20px"
  xl: "28px"
  titlebar-height: "40px"
  sidebar-width: "280px"
  transcript-width: "720px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-accent}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    height: "28px"
    padding: "0 12px"
  button-primary-active:
    backgroundColor: "{colors.accent-pressed}"
  button-primary-disabled:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ink-disabled}"
  button-secondary:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    height: "28px"
    padding: "0 12px"
  button-secondary-active:
    backgroundColor: "{colors.raised}"
  button-destructive:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.danger}"
    rounded: "{rounded.control}"
    height: "28px"
  icon-button:
    textColor: "{colors.ink-secondary}"
    rounded: "{rounded.control}"
    size: "28px"
  icon-button-hover:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ink}"
  input:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
    height: "30px"
    padding: "0 10px"
  segmented:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ink-secondary}"
    rounded: "{rounded.control}"
    padding: "2px"
  segmented-selected:
    backgroundColor: "{colors.control-selected}"
    textColor: "{colors.ink}"
    rounded: "{rounded.inset}"
    height: "24px"
  worktree-card:
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.inset}"
    padding: "8px 10px"
  worktree-card-selected:
    backgroundColor: "{colors.selected}"
    textColor: "{colors.ink-secondary}"
  message-user:
    backgroundColor: "{colors.bubble}"
    rounded: "{rounded.card}"
    padding: "10px 14px"
  composer:
    backgroundColor: "{colors.sheet}"
    rounded: "{rounded.card}"
    padding: "10px 10px 8px"
  sheet:
    backgroundColor: "{colors.sheet}"
    rounded: "{rounded.card}"
    padding: "20px"
    width: "400px"
  toast:
    backgroundColor: "{colors.sheet}"
    rounded: "{rounded.control}"
    padding: "6px 12px"
---

# Design System: Chord

## Overview

**Creative North Star: "Flat zinc chrome"**

Chord's look: Tailwind's zinc neutrals, a near-white canvas in light and pure black in dark, a slightly darker sidebar with a hairline edge, flat chrome with no bar fills, and a white (or `#111`) composer card with a 16px radius. One blue accent. A developer who has used any agent workspace finds the sidebar, the transcript and the composer where they expect them.

The developer chose this look on purpose. When a new screen needs a decision, reuse what Chord's screens already do before inventing anything.

The app keeps the platform conventions a Mac user expects: traffic lights on a transparent title bar, a full menu bar, a keyboard shortcut for every command, drag and drop from Finder, and light and dark appearance. Settings opens as a panel over the blurred window (⌘,).

**Key Characteristics:**

- Zinc neutrals, one blue accent, hairline borders; dark is pure black with a `#111` sidebar.
- 8px controls, 16px cards, no pills outside the Simulator view's button row.
- The system font at 13px for chrome, 14px for the transcript, the composer and sidebar titles.
- Worktree cards grouped under project names in the sidebar, a head with the branch and its shortcut, git counts under it, and a row per session; 44px rows in Settings.
- Light and dark use the same roles; the app follows the system by default.

## Colors

A neutral gray ladder carries the layout. Blue marks the one primary action and keyboard focus. Red and green appear as text.

Every color, shadow and radius lives in `src/ui/tokens.css` as a CSS variable named after the frontmatter key (`--canvas`, `--accent`, `--radius-control`). The dark block under `:root[data-theme="dark"]` overrides the same names with the `dark-` values. Components use the variables and hold no raw color value; `npm run check` fails on a hex color in `src/app` or `src/features`. Add a new color to both blocks.

Every value comes from Tailwind's zinc, blue, emerald, amber and red. In dark, `raised`, `selected`, `control-selected` and the lines are white at low opacity; a sticky control over scrolling content lays its wash over `canvas` so nothing shows through. The status roles keep Chord's meaning: running blue, done green. In light, running takes blue 600 and waiting, done and `git-changed` take the 700 shades, so their 11px words keep 4.5:1 on the sidebar and a tab.

### Primary

- **Action Blue** (`accent`): the fill of the primary button. One per view.
- **Pressed Blue** (`accent-pressed`): the same button while pressed.
- **Blue Ink** (`accent-ink`): blue on a neutral surface: the caret, the drop target outline, the run marker's Building and Running. Dark appearance uses a lighter blue here so it reads on graphite.
- **Blue Wash** (`accent-wash`): the text selection highlight and the halo around a recording shortcut.

### Neutral

- **Canvas** (`canvas`): the main area, the tab bar and the status bar.
- **Panel** (`panel`): the sidebar, code blocks, table headers, the Settings section list and cards.
- **Raised** (`raised`): the open tab, hovered rows, tabs and icon buttons, the segmented track, the Changes view's group headings.
- **Bubble** (`bubble`): the user's message.
- **Selected** (`selected`): the open worktree's card and every pressed neutral control.
- **Sheet** (`sheet`): the composer, the slash and picker menus, inputs, secondary buttons, sheets, the toast, the drop label.
- **Line** (`line`): the sidebar's edge and the rules between Settings rows, the border of a Settings group.
- **Strong Line** (`line-strong`): the border of an input, the composer and a secondary button.
- **Ink** (`ink`): titles, message text, selected labels.
- **Secondary Ink** (`ink-secondary`): the open worktree card's meta text, sheet descriptions, field labels, unselected segments, resting icon buttons.
- **Muted Ink** (`ink-muted`): meta lines, hints, placeholders.
- **Disabled Ink** (`ink-disabled`): the label of a disabled primary button.

### Semantic

- **Danger** (`danger`): text and icons only. The destructive button's label, the toolbar's Stop icon, the activity field's failure and error count, the Simulator view's stop recording button, the chat notice, a field hint that holds an error, the toast's warning icon, the Preview view's compile error.
- **Success** (`success`): inline text, the toolbar's Run icon, the activity field's Build succeeded and the run marker's Build succeeded.
- **Status** (`status-running`, `status-waiting`, `status-done`, `status-error`): blue, amber, green, red. Reserved for agent status marks, and the same in every place; the one other use is the fill level of the context meter and the usage bar, `status-waiting` then `status-error` (see Composer and Status bar). A status shows where you see the agent: the Working row in an open pane, a tab its group does not show, and every session row in the sidebar, the open worktree's too. An exit takes `status-error`. The mark is one component (`StatusMark`): the orbit while the agent runs, otherwise an 8px dot. The orbit is 10px: two 4px dots circle a shared center seen edge-on, once per 1.4s; the near dot grows to 115%, the far one shrinks to 50% at 40% opacity. The words come from `statusLabel` and the notification uses them too: "Working", "Awaiting input", "Done", "Failed", "Exited".
- **Git Changed** (`git-changed`): amber, the changed file count on a worktree card, and a modified or renamed file's status letter in the Changes view.
- **Diff washes** (`diff-added`, `diff-removed`): the faint green and red behind a diff tab's added and removed lines.
- **Scrim** (`scrim`): the wash behind the window's drop target and the lightbox. A sheet blurs the window behind it (12px) instead.
- **Bezel** (`bezel`): the frame around the Simulator view's screen, near black in both appearances, a step lighter than the dark canvas so it reads there.

### Named Rules

**The One Blue Rule.** A view fills one control with `accent`: its primary action. Selection stays gray (`selected`). Focus uses `accent-ink`; a link in chat keeps the text color.

**The Text-Only Semantic Rule.** Red and green color text and small icons. No red or green fill, except the stop recording button's `danger` wash and a diff's `diff-added` and `diff-removed` lines.

**The Selected Row Rule.** Meta text on a `selected` fill steps up from `ink-muted` to `ink-secondary`, so counts and shortcut hints keep a 4.5:1 contrast.

## Typography

**Body Font:** the system font (SF Pro on macOS), with Inter Variable from `@fontsource-variable/inter` as the bundled fallback (`--font-ui`)

**Code Font:** `ui-monospace` (SF Mono on macOS), then Menlo (`--font-code`), for code in chat, the editor and the terminal. `tokens.css` sets it on every `code` and `pre`; WebKit's own monospace is Courier.

**Character:** One family, four weights of emphasis: size, semibold, medium, muted color.

Sizes multiply by `--text-scale`, which the text size setting changes (⌘=, ⌘-, ⌘0). Write a size as `calc(12px * var(--text-scale))`; the root sets the 13px body.

### Hierarchy

- **Empty title** (600, 16px): the title of an empty state.
- **Heading** (600, 14px): section headings, sheet titles and the Settings header.
- **Body** (400, 13px, 1.45): descriptions, row labels, tab names. The transcript runs at 1.5.
- **Label** (500, 13px): buttons, segments, the worktree name in the title bar.
- **Code** (400, 12px): inline code, code blocks and tables of the agent's Markdown take the Body line height.
- **Meta** (400, 12px, muted): git counts, shortcut hints, helper and notice text. Field labels use 12px at 500.
- **Sidebar**: project names 11px/600 uppercase muted, card titles 13px/500, status words 11px/500, branches at 12px, git counts and shortcut hints at 11px. See `design/sidebar.md`.
- **Transcript** (14px, 1.625): messages, the user's bubble and the composer's text.

### Named Rules

**The Tabular Rule.** Numbers that change in place use `font-variant-numeric: tabular-nums`: git counts, shortcut hints, the text size percentage.

**The Title Case Rule.** Buttons, menu items and sheet titles use title case ("New Worktree", "Add Project"). Sentences in the canvas use sentence case.

**The No Ellipsis Rule.** Text never ends with "…" or "...": no button, menu item, title, placeholder or status line. Write "Add Project", "Press keys", "Loading". One exception: the chat's working verb ends with "…" ("Cooking…").

## Layout

- One window, minimum 900 × 600. A flex row of the sidebar (280px) and the main area, with the status bar under both across the full width.
- The title bar row is 40px tall (`--titlebar-height`), transparent, and drags the window. The sidebar toggle sits right after the traffic lights, whether the sidebar shows or not. The traffic lights, the toggle and the tabs share the row's center line. On a worktree the tab bars of the groups along the top take the row, at the same height; in a worktree with an Xcode project the toolbar takes it instead (see Build and Run) and the tab bars sit under it.
- The toast shows 4px below the window's top edge, centered over the title bar row.
- The transcript and the composer share a centered column, 720px at most, with 20px side padding. The composer card and a dialog fill the column's inner width, and the user's message bubble ends on the card's right edge. Transcript text, the Working row and the notice sit 15px further in (the card's border, padding and input padding), so they start where the draft text starts, at every pane width.
- The transcript fades out over its top and bottom 16px, so text scrolls away under the edges.
- The Settings panel is 960 × 760px at most and keeps 28px clear of the window edges.
- Sheets are 400px wide, cap at 80% of the window height and scroll inside.

Spacing steps: 4, 8, 12, 16, 20, 28. Rows inside a group sit 0 to 2px apart; groups sit 12px apart; a section heading gets 28px above and 8px below.

### Text selection

Text is not selectable by default. Only content someone copies or edits is: chat messages, code, file paths, error details, and the text in inputs. Mark a selectable region with `data-selectable`. `src/ui/tokens.css` sets `user-select: none` on `body` and turns it back on for inputs and `[data-selectable]`. Set both `-webkit-user-select` and `user-select`; the macOS web view ignores the unprefixed one.

## Elevation & Depth

Tonal steps carry depth inside the window: canvas, panel, raised, selected. Borders separate panels. Shadows belong to things that float above the window's content.

### Shadow Vocabulary

- **Control** (`--shadow-control`): the selected segment in a segmented control, secondary buttons.
- **Composer** (`--shadow-composer`): a soft lift under the composer card in light; none in dark.
- **Popover** (`--shadow-popover`): the toast, the toolbar's error list, the window's drop label and the chip of a dragged tab, card or project.
- **Sheet** (`--shadow-sheet`): sheets.

Popover and sheet shadows include a 1px ring: black at 8% in light, white at 10% in dark. The ring keeps a floating surface's edge visible on a dark canvas.

### Motion

- A sheet fades in and rises 8px from 98% scale over 200ms; its backdrop blur fades in with it. A sheet closes at once. The toast drops 8px into place over 200ms. Both use `--ease-out`.
- A worktree card's hover fill, its open fill and ring change over 120ms with `--ease-out`, and pressing its head or a session row scales it to 98.5%.
- A Settings section's hover fill and ink change over 120ms with `--ease-out`, and pressing it scales it to 98.5%.
- The project chevron, a step group's chevron and a Files folder's chevron turn in 120ms.
- In the chat, a message, a step row, the Working row, the find bar and the notice fade in and rise 4px over 200ms with `--ease-out` when they arrive. A chat that opens with history shows it still. Each part of a step group's summary whose text changes fades in and unblurs from 2px over 400ms with a plain ease-out.
- The slash menu and a composer picker menu fade in and rise 4px over 200ms with `--ease-out` when they open. An extension's dialog in place of the composer fades in over 120ms.
- A message's copy button and time fade in, drop 3px and unblur from 2px over 200ms with `--ease-out` on hover. They leave the same way over 120ms after a 60ms delay.
- A session row's status and the run marker fade in and rise 4px over 200ms with `--ease-out` when they change.
- The send button changes color over 120ms and presses to 97% scale.
- The agent button changes fill and ink over 120ms and presses to 97% scale. The agent menu grows out of it: it fades in, rises 4px and scales from 96% over 200ms with `--ease-out`, from the button's center on its bottom edge. A sliding Segmented control moves its thumb to the checked segment over 200ms with `--ease-out`.
- A new tab fades in and grows from 98% scale over 200ms with `--ease-out`. The tabs a worktree opens with show still.
- The context ring and the usage bar fill to a new value over 200ms with `--ease-out`.
- The lightbox and its scrim animate like a sheet.
- In the welcome, a step change slides the new step in from 16px and 2px blur over 280ms with `--ease-out`; a harness row's state and command chip fade in and rise 4px over 200ms when a check changes them, and the default's check draws its stroke over 320ms. See `design/onboarding.md`.
- In the Changes view, a row that a read brings after the view's first one, staged, unstaged or new on disk, fades in and rises 4px over 200ms with `--ease-out`. The rows the view opens with show still. A row's More button fades in and out over 120ms with `--ease-out`. The commit mode menu fades in, rises 4px and scales from 96% over 200ms with `--ease-out`, and its chevron turns in 120ms. A checkbox presses to 90% scale over 60ms, and its check fades in and grows from 80% scale over 120ms with `--ease-out`.
- The toolbar's error list fades in, drops 4px and grows from 96% over 200ms with `--ease-out`, out of the error count's corner.
- The toolbar's Stop glyph changes color over 120ms with `--ease-out` when it enables or disables.
- The toolbar's activity line slides its segment across the field every 1.4s while a build, an install or a launch goes on; under Reduce Motion the line fills the field's width and pulses to 35% opacity every 2.4s instead.
- Under Reduce Motion the sheet, its backdrop, the lightbox, the chat drop card, scrim, toast, chevrons, the context ring, the usage bar and the onboarding check's stroke skip their animation, the toolbar's error list only fades, the welcome's sidebar loop stays on its first beat, the welcome's step slide, a harness row's state and command chip, chat items, the slash, picker and agent menus, the chat drop card's content, a message's copy button and time, a session row's status, the run marker, a step summary's changed part, a new tab and a Changes view row and the commit mode menu only fade, the send button, the agent button, a sliding Segmented control's thumb, the jump button, a worktree card, a settings section and a checkbox and its check do not move or scale, the jump scrolls instantly and spinners and the orbit run at a third of their speed.

## Shapes

- **Controls** (8px, `--radius-control`): buttons, icon buttons, inputs, the segmented track, the toast.
- **Cards** (16px, `--radius-card`): the composer, the user's message, sheets, the drop target.
- **Inset** (6px, `--radius-inset`): a segment inside the segmented track, the close button inside a tab, the sidebar's worktree cards.
- **Check** (4px, `--radius-check`): the 14px checkbox of the Changes view and of a multi select dialog.

No pills, except the Simulator view's button row, which copies the simulator's own hardware look. A circle appears only as the spinner, the status dot and orbit, the send button and the jump to latest button.

## Components

Each component has a file of its own in `design/`. Read the file of every component your change touches. For a control this list does not name, run `rg -il "<control>" design/`.

- Buttons: `design/buttons.md`
- Inputs: `design/inputs.md`
- Sidebar: `design/sidebar.md`
- Tab bar and panes: `design/tab-bar-and-panes.md`
- Side panel and Files: `design/side-panel-and-files.md`
- Changes view: `design/changes-view.md`
- Simulator view: `design/simulator-view.md`
- Preview view: `design/preview-view.md`
- Bottom panel and Terminal: `design/bottom-panel-and-terminal.md`
- Editor: `design/editor.md`
- Status bar: `design/status-bar.md`
- Build and Run: `design/build-and-run.md`
- Chat: `design/chat.md`
- Sheets: `design/sheets.md`
- Onboarding and the harness rows: `design/onboarding.md`
- Settings: `design/settings.md`
- Toast: `design/toast.md`
- Drop target: `design/drop-target.md`
- Scrollbars: `design/scrollbars.md`
- Icons: `design/icons.md`

## Do's and Don'ts

### Do:

- **Do** use the four neutral steps in order: `canvas` for the work area, `panel` for the frame around it, `raised` for hover and resting controls, `selected` for the open item and pressed controls.
- **Do** give a view one blue primary button.
- **Do** keep controls at 8px and cards at 16px.
- **Do** give every command a menu bar item and a keyboard shortcut.
- **Do** put each new color in both the light and the dark block of `src/ui/tokens.css`.
- **Do** check muted text against the surface it sits on; step up to `ink-secondary` on `selected`.

### Don't:

- **Don't** color selection, hover or a second button blue.
- **Don't** fill an area with red or green.
- **Don't** use a pill shape for a button, a row or a toast.
- **Don't** add a second accent hue, a gradient, or a blur anywhere except behind a sheet.
- **Don't** add a hover color to the primary button.
- **Don't** make chrome text selectable.
- **Don't** put a raw hex color in a component.
- **Don't** end a label, title or status with "…" or "...". The chat's working verb is the one exception.
