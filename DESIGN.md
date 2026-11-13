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
  danger: "#e7000b"
  success: "#009966"
  status-running: "#2b7fff"
  status-waiting: "#e17100"
  status-done: "#009966"
  status-error: "#e7000b"
  git-changed: "#e17100"
  dark-canvas: "#000000"
  dark-panel: "#111111"
  dark-raised: "rgba(255, 255, 255, 0.05)"
  dark-selected: "rgba(255, 255, 255, 0.09)"
  dark-sheet: "#111111"
  dark-control-selected: "rgba(255, 255, 255, 0.1)"
  dark-bubble: "#1e1e1e"
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
  dark-danger: "#fb414a"
  dark-success: "#00bc7d"
  dark-status-running: "#2b7fff"
  dark-status-waiting: "#fe9a00"
  dark-status-done: "#00bc7d"
  dark-status-error: "#fb414a"
  dark-git-changed: "#fe9a00"
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
  sidebar-width: "264px"
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




The app keeps the platform conventions a Mac user expects: traffic lights on a transparent title bar, a full menu bar, a keyboard shortcut for every command, drag and drop from Finder, and light and dark appearance. Settings opens as a panel over the blurred window (⌘,).

**Key Characteristics:**

- Zinc neutrals, one blue accent, hairline borders; dark is pure black with a `#111` sidebar.
- 8px controls, 16px cards, no pills.
- The system font at 13px for chrome, 14px for the transcript, the composer and sidebar titles.
- Worktree cards grouped under project names in the sidebar, two lines each; 44px rows in Settings.
- Light and dark use the same roles; the app follows the system by default.

## Colors

A neutral gray ladder carries the layout. Blue marks the one primary action and keyboard focus. Red and green appear as text.

Every color, shadow and radius lives in `src/ui/tokens.css` as a CSS variable named after the frontmatter key (`--canvas`, `--accent`, `--radius-control`). The dark block under `:root[data-theme="dark"]` overrides the same names with the `dark-` values. Components use the variables and hold no raw color value; `npm run check` fails on a hex color in `src/app` or `src/features`. Add a new color to both blocks.


### Primary

- **Action Blue** (`accent`): the fill of the primary button. One per view.
- **Pressed Blue** (`accent-pressed`): the same button while pressed.
- **Blue Ink** (`accent-ink`): blue on a neutral surface: the caret, the drop target outline. Dark appearance uses a lighter blue here so it reads on graphite.
- **Blue Wash** (`accent-wash`): the text selection highlight and the halo around a recording shortcut.

### Neutral

- **Canvas** (`canvas`): the main area, the tab bar and the status bar.
- **Panel** (`panel`): the sidebar, code blocks, table headers, the Settings section list and cards.
- **Raised** (`raised`): the open tab, hovered rows, tabs and icon buttons, the segmented track.
- **Bubble** (`bubble`): the user's message.
- **Selected** (`selected`): the open worktree's row and every pressed neutral control.
- **Sheet** (`sheet`): the composer, the slash and picker menus, inputs, secondary buttons, sheets, the toast, the drop label.
- **Line** (`line`): the sidebar's edge and the rules between Settings rows, the border of a Settings group.
- **Strong Line** (`line-strong`): the border of an input, the composer and a secondary button.
- **Ink** (`ink`): titles, message text, selected labels.
- **Secondary Ink** (`ink-secondary`): the open worktree card's meta text, sheet descriptions, field labels, unselected segments, resting icon buttons.
- **Muted Ink** (`ink-muted`): meta lines, hints, placeholders.
- **Disabled Ink** (`ink-disabled`): the label of a disabled primary button.

### Semantic

- **Danger** (`danger`): text and icons only. The destructive button's label, the chat notice, a field hint that holds an error, the toast's warning icon.
- **Success** (`success`): inline text only.
- **Status** (`status-running`, `status-waiting`, `status-done`, `status-error`): blue, amber, green, red. Reserved for agent status marks, and the same in every place; the one other use is `status-waiting` on the context meter from 80% full. A status shows once, where you see the agent: the Working row in an open pane, the tab of a session that is not open, the card of a worktree that is not open (or of the open one while Settings covers it). An exit takes `status-error`. The mark is one component (`StatusMark`): the orbit while the agent runs, otherwise an 8px dot. The orbit is 10px: two 4px dots circle a shared center seen edge-on, once per 1.4s; the near dot grows to 115%, the far one shrinks to 50% at 40% opacity. The words come from `statusLabel` and the notification uses them too: "Working", "Awaiting input", "Done", "Failed", "Exited".
- **Git Changed** (`git-changed`): amber, the changed file count on a worktree card.
- **Scrim** (`scrim`): the wash behind the window's drop target and the lightbox. A sheet blurs the window behind it (12px) instead.

### Named Rules

**The One Blue Rule.** A view fills one control with `accent`: its primary action. Selection stays gray (`selected`). Focus uses `accent-ink`; a link in chat keeps the text color.

**The Text-Only Semantic Rule.** Red and green color text and small icons. No red or green fill.

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
- **Meta** (400, 12px, muted): git counts, shortcut hints, helper and notice text. The sidebar title and field labels use 12px at 500.
- **Sidebar**: project names 12px/500 muted, card titles 14px, status words 11px/500, branches at 12px, git counts and shortcut hints at 11px. See Components › Sidebar.
- **Transcript** (14px, 1.625): messages, the user's bubble and the composer's text.

### Named Rules

**The Tabular Rule.** Numbers that change in place use `font-variant-numeric: tabular-nums`: git counts, shortcut hints, the text size percentage.

**The Title Case Rule.** Buttons, menu items and sheet titles use title case ("New Worktree", "Add Project"). Sentences in the canvas use sentence case.

**The No Ellipsis Rule.** Text never ends with "…" or "...": no button, menu item, title, placeholder or status line. Write "Add Project", "Press keys", "Loading".

## Layout

- One window, minimum 900 × 600. A flex row: the sidebar (264px, full window height) and the main area, with the status bar under the main area.
- The title bar row is 40px tall (`--titlebar-height`), transparent, and drags the window. The sidebar toggle sits right after the traffic lights, whether the sidebar shows or not. The traffic lights, the toggle and the tabs share the row's center line. On a worktree the tab bar takes the row over the work area, at the same height.
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
- **Popover** (`--shadow-popover`): the toast, the window's drop label and a dragged tab's chip.
- **Sheet** (`--shadow-sheet`): sheets.

Popover and sheet shadows include a 1px ring: black at 8% in light, white at 10% in dark. The ring keeps a floating surface's edge visible on a dark canvas.

### Motion

- A sheet fades in and rises 8px from 98% scale over 200ms; its backdrop blur fades in with it. A sheet closes at once. The toast drops 8px into place over 200ms. Both use `--ease-out`.
- The project chevron, a step group's chevron and a Files folder's chevron turn in 120ms.
- In the chat, a message, a step row, the Working row, the find bar and the notice fade in and rise 4px over 200ms with `--ease-out` when they arrive. A chat that opens with history shows it still.
- A message's copy button and time fade in, drop 3px and unblur from 2px over 200ms with `--ease-out` on hover. They leave the same way over 120ms after a 60ms delay.
- A worktree card's status fades in and rises 4px over 200ms with `--ease-out` when it changes.
- The send button changes color over 120ms and presses to 97% scale.
- The lightbox and its scrim animate like a sheet.
- Under Reduce Motion the sheet, its backdrop, the lightbox, the chat drop card, scrim, toast and chevrons skip their animation, chat items, the chat drop card's content, a message's copy button and time and a card's status only fade, the send button and the jump button do not move or scale, the jump scrolls instantly and spinners and the orbit run at a third of their speed.

## Shapes

- **Controls** (8px, `--radius-control`): buttons, icon buttons, inputs, worktree cards, the segmented track, the toast.
- **Cards** (16px, `--radius-card`): the composer, the user's message, sheets, the drop target.
- **Inset** (6px, `--radius-inset`): a segment inside the segmented track, the close button inside a tab, the sidebar's worktree cards.

No pills. A circle appears only as the spinner, the status dot and orbit, the send button and the jump to latest button.

## Components

### Buttons

- **Primary** (`.button-primary`): blue fill, white 13px/500 label, 28px tall. One per view. No hover change, per macOS; pressed steps to `accent-pressed`.
- **Disabled primary:** `raised` fill, `ink-disabled` label. A form keeps the button disabled until its input is valid.
- **Loading:** the label hides and a 14px spinner takes its place in the same button (`aria-busy="true"`).
- **Secondary** (`.button-secondary`): `sheet` fill with a 1px `line-strong` inner border and the control shadow. Hover lays `raised` over it, pressed `selected`.
- **Destructive** (`.button-destructive`): a secondary button with a `danger` label.
- **Icon button** (`.icon-button`): 28px square, no fill at rest, `raised` on hover, `selected` when pressed. Each has a tooltip with its shortcut.
- **Icon button disabled:** `ink-disabled` glyph, no hover fill.
- **Focus:** a 2px `focus-ring` outline, 2px outside the control.

### Inputs

- **Field** (`.input`): 30px tall, `sheet` fill, 1px `line-strong` border, `ink-muted` placeholder. The label sits above at 12px/500; the hint sits below at 12px muted and reserves its line so the sheet keeps its height. A hint that holds an error (a branch git refuses, a failed create) takes `danger`.
- **Focus:** no visual change. The caret shows where you type. No outline, no blue border, no halo.
- **Segmented control** (`.segmented`): a `raised` track with 2px padding. The selected segment is a `control-selected` chip with the control shadow.
- **Shortcut recorder:** a secondary button that shows the shortcut, or "None" in muted ink. While recording it gets a `canvas` fill, a 1px `accent-ink` inset border and a 3px `accent-wash` halo, and reads "Press keys".

### Sidebar

It uses the app's roles; `src/ui/tokens.css` overrides only `selected` inside `.sidebar`: white with a 1px `selected-ring` (zinc-200 at 70%) in light, white at 6% with no ring in dark.

- 264px, full window height, `panel` fill with a 1px `line` right edge. The header holds the "Workspaces" title (12px/500 `ink-muted`) and the add button.
- **List:** one group per project, in project order, 14px apart, scrolling inside. A group is the project name (12px/500 `ink-muted`, 16px line, 10px left padding so it aligns with the card text, truncates; right-click opens New Worktree and Remove Project), then its worktree cards 2px apart. No folding.
- **Shortcuts:** ⌘1 to ⌘9 follow the cards top to bottom.
- **Worktree card:** 8px 10px padding, Inset radius, 2px between its lines, `ink-muted` meta text, two lines:
  - **Title line** (20px): the first named session of the worktree (the first 30 characters of its first user message, whitespace collapsed), or the branch while no session has a name, at 14px `ink`, which truncates; then the status slot at the line's right end. The slot stays empty on the open worktree's card, whose tabs show the status. Elsewhere it never shrinks, 11px/500 in the status color, the tooltip holds the pane's last message: the orbit and the run's age ("4m", none in the first minute) while the most urgent pane runs, an 8px dot and "Awaiting input", "Failed" or the exit; a finished pane nobody opened shows a dot and "Done" in `status-done`. A pane that is done and seen shows the time since that status began ("now", "5m", "3h", "2d") in 11px `ink-muted`, tabular.
  - **Meta line** (16px, 12px meta): the branch when the title line shows a session name, after a 12px branch icon 4px before it, which truncates, the git counts at 11px/500 tabular ("↑2 ↓1" in the meta color, then the changed files as "~33" in `git-changed`), and the shortcut hint at 11px `ink-disabled` at the right end (`ink-muted` on hover and when open); the line aligns on the text baseline, and counts and hint never shrink.
- **Card menu:** right-click opens New Worktree (for its project), Delete Worktree (not on the main checkout) and Remove Project.
- **States:** hover takes `raised` over 120ms, pressed takes `raised`. The open worktree takes the sidebar's `selected` and `selected-ring`, a 500 `ink` title and `ink-secondary` meta text. The focus ring sits inside the card, so it clears the cards above and below.

### Tab bar and panes

- **Tab bar:** the title bar row (`--titlebar-height`, 40px) at the top of the work area, on `canvas` with a 1px `line` bottom edge, drawn inside its 40px so the tabs keep the title bar's center line; 8px left padding, tabs 2px apart. One tab per session, then a plus icon button. With the sidebar hidden it starts after the sidebar toggle.
- **Tab:** 28px tall, vertically centered, 220px wide at most, Control radius, no rules. A 16px chat icon on the open tab and until an agent of the session has run, then, on a tab that is not open, the status mark of its most urgent pane, except a done you already saw, which shows the chat icon again; then the name (the first 30 characters of its focused pane's first user message, whitespace collapsed, "Session 1" style while it has none; truncates; a focused file pane keeps the name of the session's first chat pane), then a 20px close icon button that shows on the open tab, and on another tab only while the pointer is over it or focus is inside it; it keeps its space when hidden, so the tab width holds still. Padding is 10px on the left and 4px after the close button; 8px separates the name and the close button; with the close button hidden (one session left) the name takes 10px on the right. `ink-muted` text; hover takes `raised` and `ink`; the open tab takes `raised` and `ink`. The close button's hover takes `selected`. The focus ring sits inside the tab. The bar scrolls sideways when tabs overflow and scrolls the open tab into view.
- **File tab:** a tab after the session tabs for each open file: a 16px file icon, the file name (the full relative path is the tooltip), and a 20px close button that is always visible, since a file tab is closed on purpose. It looks like a session tab and takes `raised` while its file shows; the session tabs then show none open.
- **Tab drag:** a tab dragged anywhere over one of the open session's panes picks the pane edge nearest the pointer and shows that half of the pane; crossing a divider keeps the last half shown with a 2px dashed `accent-ink` outline on `accent-wash`; the dragged tab dims to 60%, and a copy of it (its icon and name on a 28px `sheet` chip with the Control radius and the popover shadow) follows the pointer, 8px below and right of it. Dropping splits the pane there and moves the dragged session's panes in; its tab goes. The open session's own tab has no drop target, so dragging it does nothing. A file tab drags the same way: the open session's panes show while it moves, and dropping shows the file in a pane there (the editor or image view, with the pane close button in a split); its tab goes. ⌘D or ⌘⇧D while a file tab shows turns it into a new session tab: the file pane, then a new agent pane with the focus.
- **Focused pane:** with the session split, the focused pane shows a 28×3px `accent` bar, fully rounded, centered 6px below its top edge. On a focus change the old bar shrinks out and the new one grows from its center while fading in, over 200ms with `--ease-out`; with reduced motion it only fades. A single pane shows none.
- **Divider:** a 1px `line` rule between panes with a 9px grab area. Hover and drag step it to `line-strong`. Double-click evens the split out. A pane keeps 10% at least.
- **Pane close:** in a split, each pane has a 28px close button 6px from its top-right corner, with a 14px X and the Control radius, sized like the sidebar's icon buttons. `ink-muted` at rest; hover takes `raised` and `ink`. Its tooltip shows the Close Pane shortcut.

### Side panel and Files

- **Side panel:** one column right of the panes, under the tab bar, 300px wide by default, on `canvas` with a 1px `line` left edge. Dragging the left edge resizes it between 200px and 640px, and it leaves the panes 320px at least; the edge has a divider's 9px grab area centered on the border and takes `line-strong` on hover and drag. Open state and width are saved per worktree. It shows one view at a time. The tab bar's right end holds the view's icon button (a window with its right column marked, the mirror of the sidebar button, tooltip "Show or Hide Files (⌘⇧E)"), which stays in place while the tabs scroll and takes `selected` while its view shows.
- **Files header:** 4px top, 6px right and 18px left padding, none at the bottom; the worktree folder's name at Label weight in `ink` that truncates and holds the full path as its tooltip, then Collapse All (two chevrons pointing in at each other, disabled while no folder is open) and Reload as 24px icon buttons 2px apart.
- **Files filter:** a Field (placeholder "Filter") under the header with 8px side padding, a 16px `ink-muted` filter icon 10px inside its left edge and the text 32px in.
- **Tree:** rows are 26px, Control radius, `ink-secondary` text, 16px more indent per level, a 12px `ink-muted` chevron (turns like a step group's; a file keeps its space and shows none), then a 16px `ink-muted` folder or file icon (`ink-secondary` on the selected row), 4px apart, then the name truncating. Hover takes `raised`, pressed takes `selected`; the selected file takes `selected` and `ink`. A folder chain reads as one row (`src/app/views`). The focus ring sits inside the row. ↑ and ↓ move the focus between rows, → opens a folder, ← closes an open folder or moves to the parent's row, Return opens a file as a click does. A filter with no match reads "No matches" in meta text.
- **Tree drag:** a file row dragged onto a chat's composer dims to 60% like a dragged tab; the composer under it takes a 2px dashed `accent-ink` outline on `accent-wash`. Dropping adds the file's relative path to the end of the draft, a space apart, and focuses the composer. A disabled composer takes no drop.
- **Tree menu:** right-click on a row opens New File and New Folder (inside a folder, beside a file), Rename, Copy Path, Copy Relative Path, Reveal in Finder and Move to Trash (⌘⌫ on the focused row); right-click on the empty tree opens New File and New Folder at the root. Move to Trash asks with a confirm sheet first.
- **Open File by Name:** ⌘⇧O opens a sheet 560px wide with 12px padding: a Field (placeholder "File name") with the focus, then the worktree's files that git does not ignore as 30px rows, 312px tall at most, scrolling inside. A row is a 16px `ink-muted` file icon, the file name in `ink-secondary`, then its folder in `ink-muted` meta text, both truncating; the full path is its tooltip. A name that starts with the typed text comes first, then a name that contains it, then a path that does; the first 100 show. The highlighted row takes `selected` and `ink`, moves with ↑ and ↓ or the pointer, and Return or a click opens the file as a click in the tree does. No match reads "No matches" in meta text. Esc or a click outside closes it.
- **Name field:** Rename turns the row's name into a 22px field (`canvas`, 1px `line-strong` border, Control radius) with the name without its extension selected; New File and New Folder show the same field as the first row inside their folder. Return or leaving the field keeps the name, Esc or an empty name drops it. A new file opens in its tab.

### Bottom panel and Terminal

- **Bottom panel:** one row under the panes and the editor, left of the side panel, 240px tall by default, on `canvas` with a 1px `line` top edge. Dragging the top edge resizes it between 120px and 800px, and it leaves the panes 160px at least; the edge has a divider's 9px grab area centered on the border and takes `line-strong` on hover and drag. Open state and height are saved per worktree.
- **Terminal:** ⌘T shows or hides the worktree's terminal in the bottom panel and focuses it when it opens. One login shell per worktree starts on its first show and keeps running, with its scrollback, while the panel is hidden or another worktree shows; typing `exit` or deleting the worktree closes the panel, and the next ⌘T starts a new shell. A 28px header with a 1px `line` bottom rule: "Terminal" at 12px/500 in `ink-secondary`, `ink` while the terminal has the focus, and at the right end a 24px Hide icon button (tooltip "Show or Hide Terminal (⌘T)"), 4px from the edge. Under it xterm.js on `canvas` with 8px top, 2px bottom and 8px left padding, its rows held to the bottom edge: `ink` text and block cursor, `editor-selection` selection, the 16 ANSI colors from the `term-*` tokens (xterm's Tango palette in dark; GitHub's light terminal palette in light, so yellow and white stay readable on white), `ui-monospace` (SF Mono) at the terminal font size, 10,000 lines of scrollback (Settings › Appearance › Terminal, 9 to 24px, 12px by default). ⌘ keys run the app's shortcuts and copy and paste; every other key goes to the shell.
- **Terminal links:** a web URL in the terminal output opens in the browser with ⌘-click; a plain click does nothing.
- **Terminal find:** ⌘F while the terminal has the focus opens the chat's find bar (Chat › Find) under the header, at the terminal's top right. Matches take `editor-match` laid over `canvas` (xterm takes only opaque colors); the current one adds a 1px `accent-ink` border and scrolls into view. Esc closes the bar and returns the focus to the terminal.

### Editor

- Fills the work area left of the side panel while a file tab is open; the panes stay mounted behind it. `canvas` fill, Code text (12px monospace, Body line height), a line number gutter in `ink-disabled` (`ink-secondary` on the cursor's line), no ring on focus, `editor-selection` for the selection, `editor-match` on the matching bracket and the `syn-*` colors for keywords, strings, numbers, types and functions; comments are `ink-muted` italic. It saves 1s after the last edit and when its tab goes.
- **Image view:** an image file (PNG, JPEG, GIF, WebP) opens in its file tab as the image, not the editor: centered on `canvas` with 24px padding, scaled down to fit and never up. It reads the file again within a second of a change on disk; a file gone from disk closes its tab.

### Status bar

A 32px strip under the main area on `canvas`, with a 1px `line` top edge; the sidebar runs past it to the window's bottom. Its items sit on the strip's center line. The 28px gear icon button that opens Settings (tooltip "Settings (⌘,)") sits at the sidebar's bottom, 8px in, in a footer row of the strip's height with the same `line` top edge, so the line runs to the window's left edge and the sidebar's right edge stops at it; the gear hides with the sidebar; ⌘, and the app menu still open Settings. The other items sit at the right with 12px padding, in meta text (12px `ink-muted`, tabular figures). The strip always shows. Subscription usage reads `42% 2h 48m  ·  59% 4d 6h`: each window's percentage and the time left to its reset in its two largest units, windows joined by a middle dot with two spaces each side, refreshed each minute. Before it, 8px away, a 32 × 4px bar (2px radius, `line-strong` track, `ink-muted` fill) shows the shortest window's percentage. The tooltip lists each window as `5h 42% (resets 14:30)`; a reset a day or more away carries its weekday. A window past its reset is hidden.

### Chat

- **User message:** right-aligned, 85% wide at most, on a `bubble` card (Card radius, 10px 14px padding). A steer the agent has not taken in yet sits at the end of the transcript as a user message at 50% opacity (tooltip "Queued"); it turns opaque once the agent takes it in. The agent's message is Markdown on the canvas; the user's stays plain text. Its images follow the text as 64px square thumbnails (Control radius, 1px `line` border, cropped to fill), 8px apart. A click opens the lightbox.
- **Message meta:** a 24px row 4px under each finished message, under the bubble's right edge for the user's, at the text's start for the agent's: a 24px copy icon button (tooltip "Copy Message"; it copies the message's text, the agent's as Markdown, and shows a check for 1.5s), then the message's time in 12px `ink-muted`, tabular: the clock today ("20:33"), "yesterday at 20:33", else the date ("May 3 at 20:33"), the year added outside the current one; its tooltip holds the full date. The row keeps its space and shows only while the pointer is over the message or focus is inside the row, fading over 120ms. A reply that never ended shows no time.
- **Lightbox:** the image at full size on the scrim, over everything (Control radius, sheet shadow); it fades in and rises like a sheet. A 28px close button sits at the top right, and with several images a previous and a next button at the left and right edges (disabled at the ends) and a "2 of 3" chip at the bottom; the buttons are 28px `canvas` squares with a 1px `line-strong` border and the popover shadow. ← and → step, Esc closes (the agent keeps running), a click outside the image closes.
- **Markdown:** blocks stack 4px apart, inside a quote too. Headings take the Heading row at every level. Lists indent 20px; a task shows a 14px `line-strong` box, with a check once done. A quote has a 2px `line-strong` rule on its left and `ink-secondary` text. A link is underlined in `ink-disabled`, in the text color (`ink` underline on hover), and takes the focus ring on keyboard focus; a click opens it in the default app, and only web and mail links open. A table has 1px `line` borders, a `panel` header row and scrolls sideways when wide. Inline code sits on a `raised` chip with the Inset radius.
- **Code block:** a `panel` card with a 1px `line` border and the Control radius. No language label. The copy icon button sits 4px from the top and right edges and shows only on hover or keyboard focus; it shows a check for 1.5s after it copied. The code scrolls sideways and never wraps.
- **Find:** ⌘F opens a bar floating at the transcript's top right (`--shadow-popover`, on `canvas` with a 1px `line-strong` border and the Control radius): a field, the position ("2 of 7", "No results"; meta, tabular), previous and next icon buttons (disabled without matches) and a close icon button. Return steps to the next match, Shift-Return to the previous, Esc or a click anywhere outside the bar closes it. Matches take an `editor-match` background; the current one adds a 2px `accent-ink` underline and scrolls to the middle.
- **Step group:** thoughts and tool calls between two messages, one row in meta text: a toggle with a chevron and what the group did by kind ("1 thought, 2 searches, edited 2 files, ran 4 commands": thoughts, files read, searches, folders listed, distinct files edited or written, commands run, other tool calls, each only when there is one; Inset radius, `raised` and `ink-secondary` on hover, `selected` when pressed), folded when the run is past it. After the count, the lines the group's finished edits and writes added ("+43", `success`) and removed ("−5", `danger`), tabular, each only when not zero; a write counts its whole content as added. While it runs it keeps the last thought (three lines at most) and the last three calls open; the toggle opens all steps. A call is its title (`ink-secondary`, `danger` when it failed) and its detail on one line. A call that ran 5 s or more, or failed, shows its run time at the line's right end ("55.4 s", "320 ms"), 11px `ink-disabled`, tabular; the toggle shows the group's total after the count once it reaches 5 s. A call the run left unfinished (stopped with Esc) has an `ink-muted` title. Images the group's tools returned sit once in a row below the group's last line, folded or open: the same 64px thumbnails, 8px below it, in call order, opening the same lightbox. No images, no row.
- **Working:** a row of meta text right above the composer while the agent runs. It floats over the foot of the transcript, so a run's start resizes nothing; the transcript's bottom fade moves up to end at the row's top, and the text under the row hides; while it shows, the transcript ends that much lower. The row holds the `status-running` orbit, a verb in `ink-secondary` ("Cooking", "Locking in", "Hold my beer", "Going feral"), the time since the run started ("14s", "2m 5s") and "esc stops". The run's start time picks the verb, so it holds for the whole run.
- **Notice:** one line of `danger` meta text above the composer.
- **Stopped:** a run the user stops with Esc ends in a left-aligned "Stopped" row of meta text in `ink-muted`, after the run's last item.
- **Sending** keeps the transcript where the reader left it; it follows the reply only when the reader sits at the end.
- **Subagent line:** when the harness has the subagents capability and an extension reports background runs, light lines end the transcript, after the last message, in the message column, with no box or border. They read as the Working row does: meta text (12px, `ink-muted`, tabular), a `StatusMark` (orbit while it runs, `status-done` or `status-error` after) and, 8px after it, one joined text. A single run is one line, "worker · bash · 4s": name, tool and time joined with " · ", each part only when known, no leading separator. Two or more leaf rows (a chain, a fan-out) get a summary line first, "3 subagents · 12s" (the count of the leaf rows, the agents that hold no others, so a chain of three steps reads 3; then the time from the first start to now or to the last end), its mark the orbit while one runs, else `status-error` when one failed, else `status-done`; the rows follow, indented 18px per level once the summary shows (what a run holds sits one level deeper; without a summary the top rows sit flush), each "name · tool · time" with its own mark. The time ticks while a run runs and freezes at its length once it ended. A chain of single children that repeat one name (a run, its step, a nested summary of the same agent) is one row at any depth: it keeps the deepest tool, the earliest start, the deepest end and one state: running while any of them runs (a nested run can outlive its parent), else failed if any failed, else done; a running row has no end, so its time keeps ticking. A stop or a turn's end keeps the lines, because background runs outlive the turn; the lines stay through an automatic context compaction; the extension's own empty report on `/new` or `/resume`, and the agent's exit, clear them. With no run reported, or once the extension clears them, nothing shows. Chord has no control that stops a background run.
- **Composer:** a `sheet` card with a 1px `line` border, the Card radius, 10px 10px 8px padding and `--shadow-composer`. The draft is 14px at 1.625. Focus inside changes nothing. The text area starts one line tall, grows with the draft up to 224px, then scrolls inside; a new pane width or text size fits it to the draft again. While the agent is not ready the text area is inert and its placeholder takes `ink-disabled`. Pasted and attached images wait above the text as 48px thumbnails with an 18px remove button on their top-right corner; a thumbnail takes the transcript thumbnail's zoom cursor and `line-strong` hover border and opens the lightbox. Its bar holds the attach icon button (paperclip, tooltip "Attach Images") on the left, shown only when the harness has the attachments capability; after it, when the harness has the models capability and has told its model, two plain text buttons side by side that read as one label: the current model's name in `ink-secondary` and, only when that model reasons, the thinking level capitalized in `ink-muted` (for example "Sonnet 5.5 Medium"): 28px tall, 8px padding on both sides, no gap, body size with no icon and no fill, `raised` on hover with the text one step brighter (`ink`, `ink-secondary`), `selected` when pressed, `ink-disabled` and inert while the agent is not ready (as the attach button), one line each, the model name ellipsizes first when the bar is narrow. A click opens that picker as a menu above the composer (see Picker menu); a second click on either button closes it. Between the model label and the send button, when the harness has the context capability and has told its window, the context meter, set off from the label by a 1px `line-strong` divider 14px tall so it never reads as part of it: a 12px ring (2px stroke, `line-strong` track, the fill in the text color) and, 6px after it, the session's fill of the window as a percentage in 12px `ink-muted` meta text, tabular, 8px padding on both sides, with a tooltip of the tokens used and the window; from 80% it takes `status-waiting`. `/new` empties it (0%). The send button sits on the right: a 32px `accent` circle with a white up arrow, `selected` with an `ink-muted` arrow while the draft is empty. Its tooltip reads Send, or Steer while the agent runs and the harness has the steer capability; without it the button stays disabled until the run ends.
- **Dialog:** a dialog an agent's extension opens (select, confirm, input) takes the composer's place as the same `sheet` card: a Heading title, then the body, and the composer returns once it is answered. Esc cancels it. A select is a list of 30px rows in `ink-secondary`, 224px tall at most, scrolling inside; the highlighted row takes `selected` and `ink`, moves with ↑ and ↓ or the pointer, and Return or a click picks it. A select opens with the current option highlighted, if it has one, and marks it with a 16px `ink-muted` check at the row's right end. Every select starts with a Field (placeholder "Filter") that narrows the rows by what they contain, and reads "No matches" in meta text when none is left. A confirm shows its message in `ink-secondary` and a secondary "No" and a primary "Yes", right-aligned; an input shows a Field and "Cancel" and "Submit".
- **Slash menu:** while the draft is only a slash and a command name, a card of its own floats above the composer card, 6px clear of it and as wide: `sheet`, the composer card's own background, a 1px `line` border, the Card radius, `--shadow-popover`, 6px padding, 372px tall at most (ten rows), scrolling inside with the global scrollbar (see Scrollbars) and keeping the highlighted row in view. Its rows are 36px with 10px padding on both sides and no dividers, one line each: a 16px `ink-muted` outline icon for the command's kind (skill, prompt, extension or session; one per kind, none per command; a session command takes the tab's chat icon), then after an 8px gap its title in `ink` at the body size (the name without a `skill:` prefix, split on `-`, `_` and `:`, each word capitalized), then a one-line `ink-muted` meta description that ends in an ellipsis, then a right-aligned `ink-muted` meta label that never shrinks and sits 24px (the 8px gap plus 16px) after the description, even when it is ellipsized: "Local file" for a skill or prompt from a file, the package name for one from a package, "Extension" for an extension command, "Session" for a session command (Model, Thinking, New, Resume, Fork, Clone, Tree, Export), which runs when sent instead of going to the agent. The highlighted row takes `selected` with the Control radius and its description and label step up to `ink-secondary`. No chevrons. A command whose name equals the typed part comes first, then names that start with it; the typed part of the title, when it matches from the start of a word ignoring case and separators, takes 600, and nothing is marked when it does not. ↑ and ↓ or the pointer move the highlight; Tab or a click completes the name and a space; Return completes it too, unless the draft already is that name, then it sends. Esc or a click anywhere outside the list closes it until the draft changes. A harness that lists no commands shows no menu. A message that began as `/skill:name` shows that text in its bubble, never the skill's body.
- **Picker menu:** the model and thinking level pickers float above the composer card, which stays in place, as the slash menu does: the same card, position, size and scrolling. Neither picker has a Field; its list takes the focus. In the model picker, typing moves the highlight as a Mac menu does: to the first row with a word that starts with the typed letters, else the first that contains them. When the typed letters match nothing, the longest matching tail of them wins, so typing "fable" right after "opus" lands on Fable without deleting anything. The rows never narrow. A 1s pause starts a new name, and the highlight stays put when nothing matches. Then come the options as 36px rows in `ink` with 10px padding, the current one marked with the dialog's check. ↑ and ↓ or the pointer move the `selected` highlight, Return or a click picks. Esc or a click anywhere outside the card closes it without a pick.
- **Jump to latest:** once the reader scrolls 72px or more above the end of the transcript, a 28px `canvas` circle with a 1px `line-strong` border, the control shadow and an `ink-secondary` down arrow sits centered 4px above the Working row's place, whether the row shows or not. It shows no status: the Working row does. Hover takes `raised` and `ink`, pressed takes `selected`. A click scrolls smoothly to the end and resumes following the reply. It fades in and rises 8px from 90% scale over 200ms with `--ease-out`, fades out the same way once the reader is back at the end, and presses to 94% scale.
- **Empty state:** a centered title, one muted line and the primary button.

### Sheets

- A `sheet` card centered over the window, which blurs 12px behind it and keeps its brightness. A click outside the sheet cancels. While a sheet is open the window behind it is inert: Tab and clicks stay in the sheet.
- **Form sheet:** a header row with the title on the left and a close icon button (X) on the right that cancels. The fields follow, then a footer with the primary button on the right. Return triggers the primary button, Escape cancels.
- **Confirm sheet:** title, one centered `ink-secondary` line (`.sheet-description`) and an optional detail in meta text, 4px apart as one block; 20px below it two equal 32px buttons: secondary "Cancel" and the destructive or primary action. Focus starts on the action, Escape cancels, and a long name or path wraps inside the sheet. The detail is selectable text.

### Settings

- A panel centered over the window, which blurs behind it as behind a sheet: `canvas`, Card radius, sheet shadow, 960 × 760px at most and 28px clear of the window edges. It rises in like a sheet. Esc, the close icon button or a click outside it closes it; a sheet opened over it closes first. The window behind it is inert.
- **Section list:** 220px on the left, `panel` with a 1px `line` right edge, 12px 8px padding, rows 2px apart: General, Appearance, Shortcuts. A row is 30px, 10px padding, Control radius, a 16px `ink-muted` icon and the name in `ink-secondary` 8px after it. Hover takes `raised` and `ink` over 120ms, pressed takes `selected`; the shown section takes `selected`, `ink` at 500 and an `ink-secondary` icon. The focus ring sits inside the row. It opens on General.
- **Header:** 48px with a 1px `line` bottom rule, 28px left padding: "Settings / General" at Heading size, "Settings" in `ink-muted`, the slash at 400 in `ink-disabled`, the section in `ink`. A close icon button sits at the right.
- **Content:** scrolls inside; 640px at most, centered, 28px side padding. Each group takes a Heading (28px above, 8px below, a status notice in meta text at its right), then a `panel` card with a 1px `line` border and the Card radius.
- **Row:** 44px at least, 10px 16px padding, a 1px `line` rule above every row but the first. The name in body `ink` on the left, a one-line meta description in `ink-muted` under it (an error takes `danger`), the control on the right.

### Toast

One surface in the title bar row, 32px tall at least, 560px wide at most. It closes after 4 seconds or on click. A failure shows a `danger` warning icon before the text; a notice shows text only.

### Drop target

While the user drags an image over a chat pane that takes attachments, a card with a 2px dashed `accent-ink` outline sits 8px inside the pane: `accent-wash` over `sheet` at 85%, so the transcript shows faintly through. In its center sit a 48px `accent` circle with a 24px image icon in `on-accent`, the title "Drop images to attach" (15px, 600) and the line "They go with your next message in this chat." in `ink-muted`. The card fades in over 200ms; its content fades in and rises 4px with it, using `--ease-out`. It closes at once. The window's folder drop stays off for images.

While the user drags a folder over the window, a 2px dashed `accent-ink` outline sits 12px inside the window over the scrim, with its label on a small `sheet` chip. Over the terminal the folder drop stays off too, and no outline shows: files dropped there paste their paths into the shell.

### Scrollbars

One global style in `tokens.css`: a 4px `ink-disabled` pill with rounded ends inside a 10px gutter, `ink-muted` on hover. WebKit sizes its length from the content, 24px at least. No track fill and no rule along the edge. The terminal's xterm draws its own scrollbar in a 14px gutter; its slider takes the same 4px pill, colors and 3px edge gap.

### Icons

Line icons drawn in `src/ui/icons.tsx` on a 16px grid with a 1.3px stroke, in the current text color. Add a new icon there in the same stroke. No text glyph stands in for an icon.

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
- **Don't** end a label, title or status with "…" or "...".
