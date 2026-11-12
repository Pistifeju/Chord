---
name: Chord
description: "A macOS workspace for coding agents in the category's standard look: neutral graphite panels, one blue accent, 8pt controls. Light and dark share one set of roles."
colors:
  canvas: "#ffffff"
  panel: "#f5f5f7"
  raised: "#ececef"
  selected: "#dfdfe4"
  sheet: "#ffffff"
  control-selected: "#ffffff"
  line: "#e2e2e6"
  line-strong: "#cfcfd5"
  ink: "#1d1d1f"
  ink-secondary: "#4a4a50"
  ink-muted: "#66666d"
  ink-disabled: "#a9a9b0"
  accent: "#0067dc"
  accent-pressed: "#0058bd"
  accent-ink: "#0067dc"
  accent-wash: "rgba(0, 103, 220, 0.16)"
  on-accent: "#ffffff"
  focus-ring: "rgba(0, 103, 220, 0.6)"
  scrim: "rgba(0, 0, 0, 0.3)"
  danger: "#c4271f"
  success: "#1a7f37"
  status-running: "#1a7f37"
  status-waiting: "#9a6700"
  status-done: "#0067dc"
  status-error: "#c4271f"
  dark-canvas: "#1e1e1e"
  dark-panel: "#252526"
  dark-raised: "#2d2d30"
  dark-selected: "#37373d"
  dark-sheet: "#2a2a2c"
  dark-control-selected: "#4a4a50"
  dark-line: "#333336"
  dark-line-strong: "#46464b"
  dark-ink: "#e6e6e6"
  dark-ink-secondary: "#bdbdbd"
  dark-ink-muted: "#98989d"
  dark-ink-disabled: "#5e5e63"
  dark-accent: "#0b6fe6"
  dark-accent-pressed: "#0a60c8"
  dark-accent-ink: "#6cb6ff"
  dark-accent-wash: "rgba(77, 163, 255, 0.26)"
  dark-focus-ring: "rgba(108, 182, 255, 0.75)"
  dark-scrim: "rgba(0, 0, 0, 0.5)"
  dark-danger: "#ff6b63"
  dark-success: "#3fb950"
  dark-status-running: "#3fb950"
  dark-status-waiting: "#d29922"
  dark-status-done: "#6cb6ff"
  dark-status-error: "#ff6b63"
typography:
  page-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.45
    letterSpacing: "-0.01em"
  empty-title:
    fontSize: "15px"
    fontWeight: 600
    lineHeight: 1.45
  heading:
    fontSize: "13px"
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
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.45
  mono:
    fontFamily: "ui-monospace, 'SF Mono', Menlo, monospace"
rounded:
  inset: "6px"
  control: "8px"
  card: "12px"
spacing:
  xxs: "4px"
  xs: "8px"
  sm: "12px"
  md: "16px"
  lg: "20px"
  xl: "28px"
  titlebar-height: "40px"
  sidebar-width: "240px"
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
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    height: "28px"
    padding: "0 12px"
  button-secondary-active:
    backgroundColor: "{colors.selected}"
  button-destructive:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.danger}"
    rounded: "{rounded.control}"
    height: "28px"
  icon-button:
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.control}"
    size: "28px"
  icon-button-hover:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ink}"
  input:
    backgroundColor: "{colors.canvas}"
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
  worktree-row:
    textColor: "{colors.ink-secondary}"
    rounded: "{rounded.control}"
    height: "30px"
    padding: "0 8px 0 20px"
  worktree-row-selected:
    backgroundColor: "{colors.selected}"
    textColor: "{colors.ink}"
  message-user:
    backgroundColor: "{colors.raised}"
    rounded: "{rounded.card}"
    padding: "8px 12px"
  composer:
    backgroundColor: "{colors.panel}"
    rounded: "{rounded.card}"
    padding: "8px"
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

**Creative North Star: "The Standard Workbench"**

Chord looks like the tools a developer already has open next to it. The window is a stack of neutral gray panels with one blue accent. A developer who has used any agent workspace finds the sidebar, the transcript and the composer where they expect them.

The developer chose this look on purpose over an invented visual world. Play the convention straight. A new screen earns its place by being clear, dense and consistent with the panels around it.

The app keeps the platform conventions a Mac user expects: traffic lights on a transparent title bar, a full menu bar, a keyboard shortcut for every command, drag and drop from Finder, and light and dark appearance. Settings opens as a page in the main window (⌘,).

**Key Characteristics:**

- Four neutral surface steps, one blue accent, hairline borders.
- 8px controls, 12px cards, no pills.
- System font at 13px, semibold for titles.
- Dense rows: 30px in the sidebar, 44px in Settings.
- Light and dark use the same roles; the app follows the system by default.

## Colors

A neutral gray ladder carries the layout. Blue marks the one primary action and keyboard focus. Red and green appear as text.

Every color, shadow and radius lives in `src/ui/tokens.css` as a CSS variable named after the frontmatter key (`--canvas`, `--accent`, `--radius-control`). The dark block under `:root[data-theme="dark"]` overrides the same names with the `dark-` values. Components use the variables and hold no raw color value; `npm run check` fails on a hex color in `src/app` or `src/features`. Add a new color to both blocks.

### Primary

- **Action Blue** (`accent`): the fill of the primary button. One per view.
- **Pressed Blue** (`accent-pressed`): the same button while pressed.
- **Blue Ink** (`accent-ink`): blue on a neutral surface: the caret, the Working spinner, the drop target outline. Dark appearance uses a lighter blue here so it reads on graphite.
- **Blue Wash** (`accent-wash`): the text selection highlight and the halo around a recording shortcut.

### Neutral

- **Canvas** (`canvas`): the main area and the fill of inputs.
- **Panel** (`panel`): the sidebar and the composer.
- **Raised** (`raised`): the user's message, hovered rows and icon buttons, secondary buttons, the segmented track.
- **Selected** (`selected`): the open worktree's row and every pressed neutral control.
- **Sheet** (`sheet`): sheets, the toast, the drop label.
- **Line** (`line`): the sidebar's edge and the rules between Settings rows.
- **Strong Line** (`line-strong`): the border of an input, the composer and a secondary button.
- **Ink** (`ink`): titles, message text, selected labels.
- **Secondary Ink** (`ink-secondary`): unselected worktree names, sheet descriptions, field labels, unselected segments.
- **Muted Ink** (`ink-muted`): meta lines, hints, placeholders, resting icon buttons.
- **Disabled Ink** (`ink-disabled`): the label of a disabled primary button.

### Semantic

- **Danger** (`danger`): text and icons only. The destructive button's label, the chat notice, the toast's warning icon.
- **Success** (`success`): inline text only.
- **Status** (`status-running`, `status-waiting`, `status-done`, `status-error`): green, amber, blue, red. Reserved for agent status marks.
- **Scrim** (`scrim`): the wash behind a sheet and the drop target.

### Named Rules

**The One Blue Rule.** A view fills one control with `accent`: its primary action. Selection stays gray (`selected`). Links and focus use `accent-ink`.

**The Text-Only Semantic Rule.** Red and green color text and small icons. No red or green fill.

**The Selected Row Rule.** Meta text on a `selected` fill steps up from `ink-muted` to `ink-secondary`, so counts and shortcut hints keep a 4.5:1 contrast.

## Typography

**Body Font:** SF Pro, the system font (`--font-ui`)
**Mono Font:** SF Mono (`--font-mono`), for code

**Character:** One family, four weights of emphasis: size, semibold, medium, muted color. The text reads like the rest of macOS.

Sizes multiply by `--text-scale`, which the text size setting changes (⌘=, ⌘-, ⌘0). Write a size as `calc(11px * var(--text-scale))`; the root sets the 13px body.

### Hierarchy

- **Page title** (600, 20px, -0.01em): the title of a page, "Settings".
- **Empty title** (600, 15px): the title of an empty state.
- **Heading** (600, 13px): section headings, sheet titles, project names.
- **Body** (400, 13px, 1.45): descriptions, row labels, tab names. The transcript runs at 1.5.
- **Label** (500, 13px): buttons, segments, the worktree name in the title bar.
- **Meta** (400, 11px, muted): git counts, shortcut hints, helper and notice text. The sidebar title and field labels use 11px at 500.

### Named Rules

**The Tabular Rule.** Numbers that change in place use `font-variant-numeric: tabular-nums`: git counts, shortcut hints, the text size percentage.

**The Title Case Rule.** Buttons, menu items and sheet titles use title case ("New Worktree", "Add Project"). Sentences in the canvas use sentence case.

**The No Ellipsis Rule.** Text never ends with "…" or "...": no button, menu item, title, placeholder or status line. Write "Add Project", "Press keys", "Loading".

## Layout

- One window, minimum 900 × 600. A flex row: the sidebar (240px) and the main area.
- The title bar row is 40px tall (`--titlebar-height`), transparent, and drags the window. The sidebar toggle sits right after the traffic lights, whether the sidebar shows or not. The traffic lights, the toggle and the tabs share the row's center line. On a worktree the tab bar takes the row over the work area, at the same height.
- The toast shows 4px below the window's top edge, centered over the title bar row.
- The transcript and the composer share a centered column, 720px at most, with 20px side padding.
- The Settings page is a centered column, 560px at most.
- Sheets are 400px wide, cap at 80% of the window height and scroll inside.

Spacing steps: 4, 8, 12, 16, 20, 28. Rows inside a group sit 0 to 2px apart; groups sit 12px apart; a section heading gets 28px above and 8px below.

### Text selection

Text is not selectable by default. Only content someone copies or edits is: chat messages, code, file paths, error details, and the text in inputs. Mark a selectable region with `data-selectable`. `src/ui/tokens.css` sets `user-select: none` on `body` and turns it back on for inputs and `[data-selectable]`. Set both `-webkit-user-select` and `user-select`; the macOS web view ignores the unprefixed one.

## Elevation & Depth

Tonal steps carry depth inside the window: canvas, panel, raised, selected. Borders separate panels. Shadows belong to things that float above the window's content.

### Shadow Vocabulary

- **Control** (`--shadow-control`): the selected segment in a segmented control.
- **Popover** (`--shadow-popover`): the toast and the drop label.
- **Sheet** (`--shadow-sheet`): sheets.

Popover and sheet shadows include a 1px ring: black at 8% in light, white at 10% in dark. The ring keeps a floating surface's edge visible on a dark canvas.

### Motion

- A sheet fades in and rises 8px over 200ms; its scrim fades with it. The toast drops 8px into place over 200ms. Both use `--ease-out`.
- The project chevron and a step group's chevron turn in 120ms.
- In the chat, a message, a step row, the Working row and the notice fade in and rise 4px over 200ms with `--ease-out` when they arrive. A chat that opens with history shows it still.
- The send button changes color over 120ms and presses to 97% scale.
- Under Reduce Motion the sheet, scrim, toast and chevrons skip their animation, chat items only fade, the send button does not scale and spinners turn at a third of their speed.

## Shapes

- **Controls** (8px, `--radius-control`): buttons, icon buttons, inputs, worktree rows, the segmented track, the toast.
- **Cards** (12px, `--radius-card`): the composer, the user's message, sheets, the drop target.
- **Inset** (6px, `--radius-inset`): a segment inside the segmented track, the close button inside a tab.

No pills. A circle appears only as the spinner.

## Components

### Buttons

- **Primary** (`.button-primary`): blue fill, white 13px/500 label, 28px tall. One per view. No hover change, per macOS; pressed steps to `accent-pressed`.
- **Disabled primary:** `raised` fill, `ink-disabled` label. A form keeps the button disabled until its input is valid.
- **Loading:** the label hides and a 14px spinner takes its place in the same button (`aria-busy="true"`).
- **Secondary** (`.button-secondary`): `raised` fill with a 1px `line-strong` inner border. Pressed steps to `selected`.
- **Destructive** (`.button-destructive`): a secondary button with a `danger` label.
- **Icon button** (`.icon-button`): 28px square, no fill at rest, `raised` on hover, `selected` when pressed. Each has a tooltip with its shortcut.
- **Focus:** a 2px `focus-ring` outline, 2px outside the control.

### Inputs

- **Field** (`.input`): 30px tall, `canvas` fill, 1px `line-strong` border. The label sits above at 11px/500; the hint sits below at 11px muted and reserves its line so the sheet keeps its height.
- **Focus:** no visual change. The caret shows where you type. No outline, no blue border, no halo.
- **Segmented control** (`.segmented`): a `raised` track with 2px padding. The selected segment is a `control-selected` chip with the control shadow.
- **Shortcut recorder:** a secondary button that shows the shortcut, or "None" in muted ink. While recording it gets a `canvas` fill, a 1px `accent-ink` inset border and a 3px `accent-wash` halo, and reads "Press keys".

### Sidebar

- `panel` fill with a 1px `line` right edge. The header holds the "Projects" title and the add button.
- **Project header:** name at 13px/600, then the chevron right after it. A long name truncates; the chevron stays. Right-click opens the project menu.
- **Worktree row:** 30px tall, three slots in a fixed order: name, git counts, shortcut hint. The name truncates with an ellipsis; the two meta slots never shrink.
- **States:** hover takes `raised`, pressed takes `selected`, the open worktree takes `selected` and `ink`. The focus ring sits inside the row, so it clears the rows above and below.
- **Pane rows:** a worktree with more than one pane shows one row per pane under its card: 24px tall, 11px `ink-muted` session name (numbered, "Session 1 · 2", when the session has several panes), indented past the card's name. Hover and pressed take the card's states. The focused pane takes `selected` and `ink`, and its card gives up the fill and keeps `ink`, so the tree shows one selected row.

### Tab bar and panes

- **Tab bar:** the title bar row (`--titlebar-height`, 40px) at the top of the work area, on `panel` with a 1px `line` bottom edge. Tabs fill its full height. One tab per session, then a plus icon button. With the sidebar hidden it starts after the sidebar toggle.
- **Tab:** flat, full bar height, 220px wide at most, no radius, a 1px `line` rule on its right edge. A 16px chat icon, or a spinner while an agent of the session runs, then the name (truncates), then a 20px close icon button that shows on the open tab, and on another tab only while the pointer is over it or focus is inside it; it keeps its space when hidden, so the tab width holds still. Padding is 12px on the left and 8px after the close button, so the X glyph sits as far from the right edge as the chat icon from the left; 8px separates the name and the close button; with the close button hidden (one session left) the name takes 12px on the right. `ink-muted` text; hover takes `raised`; the open tab takes `canvas`, `ink` and a 1px `line` rule on its left, and drops the bottom edge, so it joins the pane below. The close button's hover takes `selected`, or `raised` on the open tab so it stays visible. The focus ring sits inside the tab. The bar scrolls sideways when tabs overflow and scrolls the open tab into view.
- **Tab drag:** a tab dragged over the open session's pane edge (the outer quarter) shows that half of the pane with a 2px dashed `accent-ink` outline on `accent-wash`; the dragged tab dims to 60%. Dropping splits the pane there and moves the dragged session's panes in; its tab goes. The open session's own tab has no drop target, so dragging it does nothing.
- **Focused pane:** with the session split, the focused pane shows a 28×3px `accent` bar, fully rounded, centered 6px below its top edge. On a focus change the old bar shrinks out and the new one grows from its center while fading in, over 200ms with `--ease-out`; with reduced motion it only fades. A single pane shows none.
- **Divider:** a 1px `line` rule between panes with a 9px grab area. Hover and drag step it to `line-strong`. Double-click evens the split out. A pane keeps 10% at least.
- **Pane close:** in a split, each pane has an 18px close button 8px from its top-right corner, with a 14px X, smaller than the tab's. `ink-muted` at rest; hover takes `raised` and `ink`. Its tooltip shows the Close Pane shortcut.

### Chat

- **User message:** right-aligned, 85% wide at most, on a `raised` card. The agent's message is plain text on the canvas.
- **Step group:** thoughts and tool calls between two messages, one row in meta text: a toggle with a chevron and a count ("1 thought, 3 calls"), folded when the run is past it. While it runs it keeps the last thought (three lines at most) and the last three calls open; the toggle opens all steps. A call is its title (`ink-secondary`, `danger` when it failed) and its detail in mono on one line. A call the run left unfinished (stopped with Esc) has an `ink-muted` title.
- **Working:** a row of meta text right above the composer while the agent runs: a 10px `accent-ink` spinner, a verb in `ink-secondary` ("Cooking…", "Locking in…", "Hold my beer…", "Going feral…"), the time since the run started ("14s", "2m 5s") and "esc stops". The run's start time picks the verb, so it holds for the whole run.
- **Notice:** one line of `danger` meta text above the composer.
- **Composer:** a `panel` card with a 1px `line-strong` border. Focus inside changes nothing. Its bar holds the send button on the right: a 28px `accent` circle with a white up arrow, `raised` with `ink-disabled` arrow while the draft is empty. Its tooltip reads Send, or Steer while the agent runs.
- **Empty state:** a centered title, one muted line and the primary button.

### Sheets

- A `sheet` card centered over the scrim. A click on the scrim cancels.
- **Form sheet:** a header row with the title on the left and a close icon button (X) on the right that cancels. The fields follow, then a footer with the primary button on the right. Return triggers the primary button, Escape cancels.
- **Confirm sheet:** title, one centered `ink-secondary` line (`.sheet-description`), an optional detail in meta text, then two equal 32px buttons: secondary "Cancel" and the destructive or primary action. Focus starts on the action, Escape cancels, and a long name or path wraps inside the sheet. The detail is selectable text.

### Toast

One surface in the title bar row, 32px tall at least, 560px wide at most. It closes after 4 seconds or on click. A failure shows a `danger` warning icon before the text; a notice shows text only.

### Drop target

While the user drags a folder over the window, a 2px dashed `accent-ink` outline sits 12px inside the window over the scrim, with its label on a small `sheet` chip.

### Icons

Line icons drawn in `src/ui/icons.tsx` on a 16px grid with a 1.3px stroke, in the current text color. Add a new icon there in the same stroke. No text glyph stands in for an icon.

## Do's and Don'ts

### Do:

- **Do** use the four neutral steps in order: `canvas` for the work area, `panel` for the frame around it, `raised` for hover and resting controls, `selected` for the open item and pressed controls.
- **Do** give a view one blue primary button.
- **Do** keep controls at 8px and cards at 12px.
- **Do** give every command a menu bar item and a keyboard shortcut.
- **Do** put each new color in both the light and the dark block of `src/ui/tokens.css`.
- **Do** check muted text against the surface it sits on; step up to `ink-secondary` on `selected`.

### Don't:

- **Don't** color selection, hover or a second button blue.
- **Don't** fill an area with red or green.
- **Don't** use a pill shape for a button, a row or a toast.
- **Don't** add a second accent hue, a gradient or a blur.
- **Don't** add a hover color to the primary button.
- **Don't** make chrome text selectable.
- **Don't** put a raw hex color in a component.
- **Don't** end a label, title or status with "…" or "...".
