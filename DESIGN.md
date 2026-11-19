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


### Primary

- **Action Blue** (`accent`): the fill of the primary button. One per view.
- **Pressed Blue** (`accent-pressed`): the same button while pressed.
- **Blue Ink** (`accent-ink`): blue on a neutral surface: the caret, the drop target outline, the run marker's Building and Running. Dark appearance uses a lighter blue here so it reads on graphite.
- **Blue Wash** (`accent-wash`): the text selection highlight and the halo around a recording shortcut.

### Neutral

- **Canvas** (`canvas`): the main area, the tab bar and the status bar.
- **Panel** (`panel`): the sidebar, code blocks, table headers, the Settings section list and cards.
- **Raised** (`raised`): the open tab, hovered rows, tabs and icon buttons, the segmented track.
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
- **Sidebar**: project names 11px/600 uppercase muted, card titles 13px/500, status words 11px/500, branches at 12px, git counts and shortcut hints at 11px. See Components › Sidebar.
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
- **Popover** (`--shadow-popover`): the toast, the window's drop label and the chip of a dragged tab, card or project.
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
- The model and thinking level pickers change fill and ink over 120ms and press to 97% scale.
- A new tab fades in and grows from 98% scale over 200ms with `--ease-out`. The tabs a worktree opens with show still.
- The context ring and the usage bar fill to a new value over 200ms with `--ease-out`.
- The lightbox and its scrim animate like a sheet.
- In the Changes view, a row that a read brings after the view's first one, staged, unstaged or new on disk, fades in and rises 4px over 200ms with `--ease-out`. The rows the view opens with show still. A row's More button fades in and out over 120ms with `--ease-out`. A checkbox presses to 90% scale over 60ms, and its check fades in and grows from 80% scale over 120ms with `--ease-out`.
- The toolbar's activity line slides its segment across the field every 1.4s while a build, an install or a launch goes on; under Reduce Motion the line fills the field's width and pulses to 35% opacity every 2.4s instead.
- Under Reduce Motion the sheet, its backdrop, the lightbox, the chat drop card, scrim, toast, chevrons, the context ring and the usage bar skip their animation, chat items, the slash and picker menus, the chat drop card's content, a message's copy button and time, a session row's status, the run marker, a step summary's changed part, a new tab and a Changes view row only fade, the send button, the model and thinking level pickers, the jump button, a worktree card, a settings section and a Changes view checkbox and its check do not move or scale, the jump scrolls instantly and spinners and the orbit run at a third of their speed.

## Shapes

- **Controls** (8px, `--radius-control`): buttons, icon buttons, inputs, the segmented track, the toast.
- **Cards** (16px, `--radius-card`): the composer, the user's message, sheets, the drop target.
- **Inset** (6px, `--radius-inset`): a segment inside the segmented track, the close button inside a tab, the sidebar's worktree cards.
- **Check** (4px, `--radius-check`): the Changes view's 14px checkbox.

No pills, except the Simulator view's button row, which copies the simulator's own hardware look. A circle appears only as the spinner, the status dot and orbit, the send button, the jump to latest button and the Simulator view's screenshot, record and Appearance buttons.

## Components

### Buttons

- **Primary** (`.button-primary`): blue fill, white 13px/500 label, 28px tall. One per view. No hover change, per macOS; pressed steps to `accent-pressed`.
- **Disabled primary:** `raised` fill, `ink-disabled` label. A form keeps the button disabled until its input is valid.
- **Loading:** the label hides and a 14px spinner takes its place in the same button (`aria-busy="true"`).
- **Secondary** (`.button-secondary`): `sheet` fill with a 1px `line-strong` inner border and the control shadow. Hover lays `raised` over it, pressed `selected`, both over 120ms.
- **Destructive** (`.button-destructive`): a secondary button with a `danger` label.
- **Icon button** (`.icon-button`): 28px square, no fill at rest, `raised` on hover, `selected` when pressed, its icon pressing to 90% scale over 60ms (none under Reduce Motion); the text size steppers, a tab's close button and the Simulator view's buttons press their icon the same way. Each has a tooltip with its shortcut.
- **Icon button disabled:** `ink-disabled` glyph, no hover fill.
- **Focus:** a 2px `focus-ring` outline, 2px outside the control.

### Inputs

- **Field** (`.input`): 30px tall, `sheet` fill, 1px `line-strong` border, `ink-muted` placeholder. The label sits above at 12px/500; the hint sits below at 12px muted and reserves its line so the sheet keeps its height. A hint that holds an error (a branch git refuses, a failed create) takes `danger`.
- **Focus:** no visual change. The caret shows where you type. No outline, no blue border, no halo.
- **Segmented control** (`.segmented`): a `raised` track with 2px padding. The selected segment is a `control-selected` chip with the control shadow.
- **Shortcut recorder:** a secondary button that shows the shortcut, or "None" in muted ink. While recording it gets a `canvas` fill, a 1px `accent-ink` inset border and a 3px `accent-wash` halo, and reads "Press keys".

### Sidebar

It uses the app's roles; `src/ui/tokens.css` overrides `selected` inside `.sidebar`: white with a 1px `selected-ring` (zinc-200 at 70%) in light, white at 6% with no ring in dark, and adds `card-hover`: `raised` in light, white at 3% in dark.

- 280px, full window height, `panel` fill with a 1px `line` right edge.
- **Navigator:** under the title bar row, a Segmented control across the sidebar, 8px in, 4px above the view: one icon segment per view, each a third or a quarter of the track with its 16px icon centered in `ink-muted` (`ink` on hover and when chosen), tooltip with its shortcut, as in Xcode's navigator. Workspaces (two stacked cards, ⌃1), Files (a folder, ⌃2, and ⌘⇧E), Changes (the branch, ⌃3, and ⌘⇧G), and in a worktree with an Xcode project Tests (a diamond with a check, ⌃4). The sidebar shows the chosen view between the control and the footer; without an open worktree only Workspaces shows. ⌘⇧E and ⌘⇧G go back to Workspaces while their view shows, and any of the shortcuts shows a hidden sidebar. While any agent awaits input, the Workspaces segment carries a 6px `status-waiting` dot at its icon's top right with a 1.5px `raised` ring, gone while Workspaces shows, so a card the sidebar hides still calls. The choice lasts until the app quits.
- **Files and Changes in the sidebar:** the Files view and the Changes view (see Side panel and Files, Changes view) as they were in the side panel, on the sidebar's `panel` fill; the Changes view's sticky headings take `panel` too.
- **Tests:** the Files view's header with "Tests" and a disabled Run All Tests icon button (the Tests icon), then "Chord does not run tests yet." in meta text. No test list until Chord runs tests.
- **Workspaces:** the header is a 32px row, 4px above the list: the "Workspaces" title (Label weight in `ink`, as the Files and Tests headers' title) 18px in, on the project names' edge, and the 28px add icon button with its 16px plus in `ink-muted` (`ink` on hover), its glyph on the shortcut hints' right edge.
- **List:** one group per project, in project order, scrolling inside. From the second group on, a 1px `line` rule sits 12px under the previous group and 8px above the next. A group is the project name (11px/600 `ink-muted`, uppercase, 0.06em letter spacing, 16px line, 6px above and below, 10px left padding so it aligns with the card text, truncates; right-click opens New Worktree and Remove Project), then its worktree cards 2px apart. No folding.
- **Shortcuts:** ⌘1 to ⌘9 follow the cards top to bottom.
- **Worktree card:** one Inset-radius card holds the worktree's head and its session rows. Only the head and the rows take clicks. The head has 8px 10px padding, 2px between its lines, `ink-muted` meta text, one or two lines:
  - **Title line** (20px): the branch at 13px/500 `ink`, which truncates, and the shortcut hint at 11px `ink-muted` at the right end (`ink-secondary` on hover and when open); the line centers its items, and the hint never shrinks.
  - **Meta line** (16px, 12px meta), only when there are counts or a run marker: the git counts at 11px/500 tabular ("↑2 ↓1" in the meta color, then the changed files as "~33" in `git-changed`), which never shrink, then the worktree's run marker (see Build and Run).
- **Session rows:** inside the card under the head, 4px bottom padding after the last, one 26px row per chat tab of the worktree, in layout order, 10px side padding so the name starts on the branch's edge, Inset radius, 8px between its name and its status slot. The name is the tab's name (see Tab bar and panes) at 13px `ink-muted`, which truncates. A click opens the worktree and that tab in its group, with the focus. Hover only brightens the name to `ink` over 120ms, with no fill; the open worktree's focused tab takes `ink` at 500. The status slot sits at the row's right end, on every worktree's rows, open or not. It never shrinks, 11px/500 in the status color, the tooltip holds the pane's last message: the orbit and the run's age ("4m", none in the first minute) while its agent runs, an 8px dot and "Awaiting input", "Failed" or the exit; a finished session nobody opened shows a dot and "Done" in `status-done`. A session that is done and seen shows the time since that status began ("now", "5m", "3h", "2d") in 11px `ink-muted`, tabular.
- **Reorder:** dragging a project's name moves the project with its cards; dragging a card's head moves the card within its project, never into another. A press becomes a drag after 5px, so a click still selects. The dragged group or card dims to 60%, the tab drag's chip with its name follows the pointer, and a 2px `accent-ink` line with a 1px radius marks where it lands: between two cards, or in the space between two groups. Dropping right before or after itself shows no line and changes nothing. Near the list's top or bottom edge the list scrolls. The main checkout's card moves like any other, and ⌘1 to ⌘9 follow the new order. A new worktree goes to the end of its project.
- **Card menu:** right-click opens New Worktree (for its project), Rename and Delete Worktree (neither on the main checkout) and Remove Project.
- **Branch name field:** Rename, or a double click on the branch name, turns the head into a 20px field (`canvas`, 1px `line-strong` border, Control radius) with the branch at 13px/500 selected, its text on the branch's edge. Git's reason for a name it refuses sits under it as a field hint in `danger`. Return renames the branch, Esc or leaving the field drops it.
- **States:** hover over a card lays `card-hover` on the whole card over 120ms; pressing the head or a session row scales the card to 98.5% over 120ms; the head and the rows take no fill of their own. Hover on the head brings the shortcut hint to `ink-secondary`. The open worktree's whole card, head and rows, takes the sidebar's `selected` and `selected-ring` and `ink-secondary` meta text. The focus ring sits inside the head or the row, so it clears its neighbors.

### Tab bar and panes

- **Tab bar:** each tab group has its own, 40px tall (`--titlebar-height`) over the tab it shows, on `canvas` with a 1px `line` bottom edge, drawn inside its 40px so the tabs keep the title bar's center line; the first tab starts at the bar's left edge, tabs side by side with no gap. A group's tabs, then a plus icon button that opens a new session in that group; with no tab its glyph starts 10px in, as a tab's icon does. A bar along the window's top is the title bar row and drags the window; with the sidebar hidden, the top left bar starts after the sidebar toggle. Under the toolbar a bar is 34px tall and drags nothing. In a split, each group's bar sits over its own pane, so a bar in a lower group sits mid-window.
- **Tab:** the bar's full height above its bottom line, 220px wide at most, square corners, no rules. A 16px icon of its harness (pi's logo, its 4 × 4 pixel grid in the text color) on its group's open tab and until its agent has run, then, on a tab its group does not show, the status mark of its agent, except a done you already saw, which shows the harness icon again; then the name (the first 30 characters of its first user message, whitespace collapsed, "Session 1" style while it has none; truncates), then a 20px close icon button that shows on the group's open tab, and on another tab only while the pointer is over it or focus is inside it; it keeps its space when hidden, so the tab width holds still. Padding is 10px on the left and 4px after the close button; 8px separates the name and the close button; with the close button hidden (one tab left) the name takes 10px on the right. `ink-muted` text; hover takes `raised` and `ink`; the group's open tab takes `raised` and `ink`. The close button's hover takes `selected`. The focus ring sits inside the tab. When its tabs overflow, a bar never scrolls: the tabs shrink and their names truncate.
- **File tab:** a file opens as a tab of the focused group, or opens the tab that already shows it. A click in the tree, Open File by Name and a build error open it as the group's preview tab, in place of the group's last preview tab; its name is italic. A double click on the tree row or the tab, or the first edit, keeps it, and the next file opens in a new tab. A new file opens kept. The tab shows a 16px file icon, the file name (the full relative path is the tooltip), and a 20px close button that is always visible, since a file tab is closed on purpose. It looks like a session tab. Closing the last tab of all leaves its group empty: the tab bar over the chat empty state, "No tabs open", "Open a session in this worktree." and a New Session button. Close Tab (⌘⇧W) is disabled while no tab has the focus.
- **Diff tab:** a click on a Changes view row opens the file's diff as the group's preview tab, as a click in the tree opens a file; a double click keeps it. It shows a 16px file icon with a plus and a minus, the file name, and the path with "(Staged)" or "(Changes)" as its tooltip. The pane holds a read-only CodeMirror with the editor's look and the file's syntax colors: the hunks one after another, the old and the new line number in two gutters in `ink-disabled`, then a sign gutter with "+" in `success` and "−" in `danger`; added lines take `diff-added`, removed lines `diff-removed`, across the gutters too, and their numbers step up to `ink-muted`. Before each hunk, the first too when it starts past line 1 or has buttons, a 24px `panel` band across the gutters and the code marks the unchanged lines git left out. It holds the line git names after the hunk's range, mostly the function the hunk sits in ("export function DiffView"), in `ink-muted` Code text, 8px in, on one line that truncates, as in VS Code and GitHub; with no such line the band stays empty. At the band's right end, 8px in and kept in view on the band's `panel` fill while the code scrolls sideways, the hunk's buttons, 4px apart: Discard and Stage in a Changes diff, Discard and Unstage in a Staged diff. Each is a compact secondary button: its `sheet` fill, `line-strong` inner border, control shadow, hover and pressed, at 20px tall, 8px side padding, the Inset radius and 12px/500, with its focus ring on its edge so it stays inside the band. Stage and Unstage turn `ink-disabled` with no shadow and no hover once clicked, until the next read redraws the band. A conflicted file's diff has none. It reads again with each read of the Changes view and keeps its scroll. A binary file reads "Binary file", a diff with no hunk left "No changes", and a failed first read "No diff to show" after its failure toast, as a centered meta line at 40% of the pane's height.
- **Browser tab:** a web link opens as a tab of the focused group with a 16px globe icon, the page title (its host while it has none; the address is the tooltip) and an always visible close button, as a file tab. Its pane holds a 40px bar on `canvas` with a 1px `line` bottom edge: Back, Forward and Reload icon buttons, a 28px address field that fills the row (Return opens what it holds, a bare host over https, a local one over http; Esc puts the page's address back), and Open in Default Browser. The page is a web view of its own over the rest of the pane; it hides while a sheet, Settings or a tab drag covers the window, since nothing the window draws can sit above it.
- **Tab drag:** a tab dragged over a group's tab bar marks the whole group, and dropping adds the tab to the end of that bar. Dragged over a group's pane, it picks the pane edge nearest the pointer and shows that half of the pane, and dropping opens a new group there with the tab. The mark is a 2px dashed `accent-ink` outline on `accent-wash`; crossing a divider keeps the last mark shown. The dragged tab dims to 60%, and a copy of it (its icon and name on a 28px `sheet` chip with the Control radius and the popover shadow) follows the pointer, 8px below and right of it. Its own group's bar, and its own group's edges while it is the group's only tab, show no mark, so dropping there does nothing. A group left without tabs goes. ⌘D or ⌘⇧D opens a new group with a new session beside the focused group, a file's too.
- **Focused pane:** with the groups split, the focused group's pane shows a 28×3px `accent` bar, fully rounded, centered 6px below its tab bar. On a focus change the old bar shrinks out and the new one grows from its center while fading in, over 200ms with `--ease-out`; with reduced motion it only fades. A single pane shows none.
- **Divider:** a 1px `line` rule between panes with a 9px grab area. Hover and drag step it to `line-strong`. Double-click evens the split out. A pane keeps 10% at least.

### Side panel and Files

- **Side panel:** in a worktree with an Xcode project, one column right of the panes, under the toolbar to the status bar, 300px wide by default, on `canvas` with a 1px `line` left edge. Dragging the left edge resizes it between 200px and 640px, and it leaves the panes 320px at least; the edge has a divider's 9px grab area centered on the border and takes `line-strong` on hover and drag. Open state and width are saved per worktree. It shows one view at a time, saved per worktree: the Simulator or the Preview view. Files and Changes live in the sidebar's navigator; a panel saved on one of them closes. The toolbar's right end holds one icon button per view: the Simulator view's phone (tooltip "Show or Hide Simulator (⌘⇧S)"), then the Preview view's eye (tooltip "Show or Hide Preview (⌘⇧P)"). The buttons share one capsule: 28px tall, a 1px `line` edge, Control radius, each button 30px wide with a 1px `line` between them and its focus ring inside. A button takes `selected` while its view shows. A click or the shortcut hides the panel while it shows that view, and otherwise opens the panel on it.
- **Files header:** 4px top, 6px right and 18px left padding, none at the bottom; the worktree folder's name at Label weight in `ink` that truncates and holds the full path as its tooltip, then Collapse All (two chevrons pointing in at each other, disabled while no folder is open) and Reload as 24px icon buttons 2px apart.
- **Files filter:** a Field (placeholder "Filter") under the header with 8px side padding, a 16px `ink-muted` filter icon 10px inside its left edge and the text 32px in.
- **Tree:** rows are 26px, Control radius, `ink-secondary` text, 16px more indent per level, a 12px `ink-muted` chevron (turns like a step group's; a file keeps its space and shows none), then a 16px `ink-muted` folder or file icon (`ink-secondary` on the selected row), 4px apart, then the name truncating. Hover takes `raised`, pressed takes `selected`; the selected file takes `selected` and `ink`. A folder chain reads as one row (`src/app/views`). The focus ring sits inside the row. ↑ and ↓ move the focus between rows, → opens a folder, ← closes an open folder or moves to the parent's row, Return opens a file as a click does. A filter with no match reads "No matches" in meta text.
- **Tree drag:** a file row dragged onto a chat's composer dims to 60% like a dragged tab; the composer under it takes a 2px dashed `accent-ink` outline on `accent-wash`. Dropping adds the file's relative path to the end of the draft, a space apart, and focuses the composer. A disabled composer takes no drop.
- **Tree menu:** right-click on a row opens New File and New Folder (inside a folder, beside a file), Rename, Copy Path, Copy Relative Path, Reveal in Finder and Move to Trash (⌘⌫ on the focused row); right-click on the empty tree opens New File and New Folder at the root. Move to Trash asks with a confirm sheet first.
- **Open File by Name:** ⌘⇧O opens a sheet 560px wide with 12px padding: a Field (placeholder "File name") with the focus, then the worktree's files that git does not ignore as 30px rows, 312px tall at most, scrolling inside. A row is a 16px `ink-muted` file icon, the file name in `ink-secondary`, then its folder in `ink-muted` meta text, both truncating; the full path is its tooltip. A name that starts with the typed text comes first, then a name that contains it, then a path that does; the first 100 show. The highlighted row takes `selected` and `ink`, moves with ↑ and ↓ or the pointer, and Return or a click opens the file as a click in the tree does. No match reads "No matches" in meta text. Esc or a click outside closes it.
- **Name field:** Rename turns the row's name into a 22px field (`canvas`, 1px `line-strong` border, Control radius) with the name without its extension selected; New File and New Folder show the same field as the first row inside their folder. Return or leaving the field keeps the name, Esc or an empty name drops it. A new file opens in its tab.

### Changes view

- **Scope:** while the worktree has a base, a header over the list, 4px 8px 0 padding: a Segmented control as wide as the panel with Uncommitted and Branch, each segment half of it, its label truncating in a narrow panel. Under Branch, 4px under the control, a 28px row 8px in: "Compared to" in 12px `ink-muted`, which never wraps, then the base as a 24px button (12px/500 `ink-secondary`, 6px padding, Inset radius, `raised` and `ink` on hover over 120ms with `--ease-out`, `selected` pressed, tooltip "Base Branch") with a 12px `ink-muted` down chevron 4px after the name, which truncates. It opens a native menu under itself, as the run menu: the default base, then under disabled "Local" and "Remote" lines, each after a separator, the other branches; the base in use carries the check mark. Branch has no checkboxes, More buttons or commit area.
- **Branch:** two groups with the Staged and Changes headings' look, without checkboxes, their names 8px in: Commits, then Files, each with its count; empty they read "No commits" and "No changes". A commit row is a Files tree row: its 12px chevron 8px in (turns like a folder's), 4px after it the subject in `ink-secondary`, which truncates, then the short hash at 11px in the code font and the age ("3h", as a session row's) in 12px, both `ink-muted`; its tooltip is the subject. A click, → and ← open and close it to the files it changed against its first parent, 24px in, on the subject's edge. A file row is the Changes row without its checkbox and More button, 8px in under Files; a click opens its diff in the preview tab, for the branch since the merge base or for its commit, read-only without hunk buttons; the tab's tooltip adds "(Compared to main)" or "(Commit a41c9e2)". ↑ and ↓ move the focus between rows and open a file's diff, as under Uncommitted.
- **Commit:** under the groups: a 1px `line` top edge and 12px padding, its parts 10px apart. First, while the branch has commits its upstream lacks or no upstream and the repository has a remote, the push line: "↑2 to origin/main" in 12px `ink-muted` with the count at 500 in `ink-secondary`, tabular ("First push to origin/feat" before the upstream exists), truncating, then Push at its right end. Then the message card, as the composer in small: `sheet` fill, a 1px `line-strong` border, Control radius. Inside it the message field with no border of its own, 8px 10px 4px padding, Body line height, placeholder "Message", three lines tall empty and growing with the text to eight, then scrolling; Return adds a line and ⌘Return commits; its tooltip is the author git records. Under the text, right-aligned at 11px tabular `ink-muted`, the first line's length as "34/50", `git-changed` past 50, empty while the field is. The card's foot, 4px padding: the Amend toggle (shown while HEAD has a commit) and the Push After toggle (shown while the branch has a remote, tooltip "Push to origin/main after the commit"), each 24px, a 14px icon and a 12px/500 label in `ink-muted`, Inset radius, `raised` and `ink` on hover, `selected` and `ink` while on, over 120ms; then Commit at the right end, a primary button at 24px with the Inset radius, reading "Amend" while Amend is on, tooltip "Commit (⌘Return)", enabled while the field holds text and a file is staged or Amend is on. Push is the primary button while Commit cannot run, and secondary otherwise, so one blue shows. Amend fills an empty field with the last commit's message, and taking it off empties the field while that message stands unchanged. A running commit or push shows the loading spinner in its button (in `ink-secondary` on the secondary one) and takes no click until it ends. A commit empties the field and takes Amend off; with Push After on, the push follows it. Both read the view and the cards' git counts again when they end.
- **Groups:** the worktree's changed files from `git status`, under Staged (the index against HEAD), then Changes (the file on disk against the index); both groups always show, so an empty one reads its 0 count; under its heading it reads "No staged files" or "No changes" in meta text on the rows' name edge, its checkbox is disabled (a `line` border, no fill). A group's heading is a 26px row, 8px left and 4px right padding: its name at 12px/500 `ink-secondary`, then 6px after it the file count in `ink-muted`, tabular. The heading starts with its checkbox, 8px in, the name 8px after it on the rows' name edge: checked on Staged, where a click unstages every file (tooltip "Unstage All"), empty on Changes, where it stages every file ("Stage All"). Groups sit 12px apart. A heading sticks to the list's top on `canvas` while its group's rows scroll under it, until the next heading pushes it out, as in VS Code. The list scrolls inside with the Files tree's padding, its hover and its focus ring inside the row; ↑ and ↓ move the focus between rows and open each one's diff as a click does, across both groups, Return too, and the selected row scrolls into view.
- **Checkbox:** 14px, Check radius, a 1px `line-strong` border on `sheet`, `ink-muted` border on hover; checked takes an `ink` fill and border with a 12px check in `canvas`, a neutral look, since a blue box on every row would break the One Blue Rule. Fill and border change over 120ms with `--ease-out`.
- **Row:** a Files tree row in path order: its checkbox 8px in, checked under Staged and empty under Changes, where a click stages or unstages the file and Space does the same on the focused row, then moves the focus on and opens the diff of the next row in the group (the one before when it was last, the file itself in its new group when the group is left empty); the checkbox takes no Tab stop of its own. Then, 30px in, the file name in `ink-secondary` (struck through for a deleted file; it truncates past 70% of the row), 6px after it the folder in 12px `ink-muted` (`ink-secondary` on the selected row), which truncates, then at the right end the lines added ("+12", `success`) and removed ("−3", `danger`) at 11px/500, tabular, each only when not zero, and the status letter at 11px/600 in a 12px column: M modified and R renamed in `git-changed`, A added and U untracked in `success`, D deleted and ! conflicted in `danger`; its tooltip is the status word. The row's tooltip is its path, a rename's as "old → new". The row whose diff the focused tab shows takes `selected` and `ink`. Right of the status letter, 4px from the row's end, a 20px More icon button (three dots, Inset radius, tooltip "More") shows while the pointer or the focus is on the row and keeps its space when hidden; `selected` on hover, as a tab's close button. It opens the row's menu, as right-click does: Stage and Discard under Changes, Unstage and Discard under Staged; a conflicted row, or a pick that holds one, has no Discard, since git cannot restore it.
- **Pick:** a shift-click, or ↑ and ↓ with shift, picks the rows from the last plainly clicked or arrowed row to this one, within its group; a shift-click opens no diff. The picked rows take `selected` and `ink`, in place of the row whose diff the focused tab shows. A picked row's checkbox, Space, More button and right-click act on every picked row ("Stage 3 files" as the checkbox's label). A plain click or arrow picks one row again, Esc drops the pick, and a row that moves to the other group or goes leaves it.
- **Discard:** Discard and a hunk's Discard ask with a confirm sheet first ("Discard Changes?" for one file, "Discard 3 Files?" for more, "Discard Hunk?"), naming the file and its path; then Chord restores the file or the hunk from the index, a staged file or hunk from the last commit, and moves a file git does not track or a staged new file to the Trash (the button reads "Move to Trash").
- **Stashes:** at the Changes heading's right end, a 20px Stash Changes icon button (a box, Inset radius, tooltip "Stash Changes") stashes the staged, unstaged and untracked changes, then shows a notice toast ("Stashed 3 files", a file both staged and changed counting once); it is disabled while both groups are empty. Under Changes, while the repository has stashes, a Stashes group: its heading looks like the others, with a 12px `ink-muted` chevron in the checkbox's place, so the name sits on the same edge, and the whole heading is a button (`raised` on hover, focus ring inside) that opens and closes the group, as → and ← do; the group starts closed and holds its state per worktree until the app quits, so 30 stashes cost one row. Open, it lists the stashes newest first, each a Branch commit row with the message in place of the subject, then the branch it came from in 12px `ink-muted` (40% of the row at most, truncating), then the age; git's "WIP on main:" leaves the message. A stash opens like a commit to the files it holds, the untracked ones with U; a click opens the file's diff read-only, its tab's tooltip adding "(Stash a41c9e2)". Right-click on a stash: Apply, Pop, a separator, Drop. Drop asks with a confirm sheet ("Drop Stash?", the message as its detail, the destructive button "Drop"). A failed or conflicting Apply or Pop shows git's message in a failure toast and leaves the stash in the list.
- **Reads:** git is read when the view shows and with each git counts refresh while it shows, so on window focus and every 10 seconds. A failed read shows git's message in a failure toast and keeps the list.

### Simulator view

- **Screen:** the worktree's simulator, live, as large as fits with its bezel and its buttons 12px inside the view, keeping its shape; the screen and the buttons under it center as one group. The toolbar's phone button names the view. Its corners round by 12% of its width. Chord draws it natively under the window's web content, which opens a hole over it, so a toast, a menu or the scrim paints over the screen. A sheet or Settings closes the hole: their blur cannot reach the screen, and the canvas shows in its place until they close.
- **Buttons:** one row centered 16px under the bezel: Home, Lock and Shake in a 32px pill with a 1px `line-strong` edge, icons only, each button 40 × 28px with a 16px icon; 8px after it the screenshot button, 8px after that the record button and 8px after that the Appearance button, each in a 32px circle with the same edge, its button 28px round. Each button sits 1px inside the edge, so its fill never touches the hairline. Each button shows an `ink-secondary` icon; tooltips "Home (⌘⇧H)", "Lock (⌘L)", "Shake (⌃⌘Z)", "Take Screenshot (⌘S)", "Record Screen" (while it records "Stop Recording"), "Appearance". Each takes a `raised` pill on hover and `selected` when pressed, with its icon at 90%; the focus ring sits inside the button. They show only with the screen. When the lock icon turns between Lock and Unlock, its shackle closes or opens over 300ms and the icon tips 8° and back over 350ms, both easing out. A Shake rocks the phone icon 15° side to side, dying out over 450ms. Chord draws them natively with the screen, so they never slide against it on a resize.
- **Appearance:** the last circle in the button row (half-filled circle icon). It opens a native menu: Light and Dark (the current one checked), then the submenus Text Size (the Dynamic Type sizes, extra small to accessibility extra extra extra large), Language and Region, each with the current choice checked and named in the Mac's language. Light, Dark and the text size change the simulator at once; a language or region applies to an app the next time it launches. The Preview view has it too, for its preview simulator.
- **Input:** a click or drag on the screen is a touch; a drag that leaves the screen keeps touching its edge, and one that starts on the bottom 3% is a swipe from the bottom edge. A trackpad pinch or rotate over the screen is a two-finger pinch or rotate. ⌥-drag pinches and rotates around the screen's center, ⌥⇧-drag moves two fingers together. While ⌥ is held over the screen, two 22px circles (`sheet` at 60%, 1px `line-strong` border) show where the fingers go down.
- **Bezel:** an 8px `bezel` frame around the screen, its corners following the screen's, with a 1px `line-strong` edge outside it, as a phone's. Chord draws it natively with the screen, so the frame never slides against the screen on a resize. It shows the same with or without the focus: no focus ring.
- **Screenshot:** the circle in the button row takes a shot. A shot leaves a thumbnail at the view's left, 8px from its edge, its bottom level with the button row's bottom: the screenshot at most 56px wide and 96px tall, on `sheet` with a 1px `line-strong` edge, Inset radius and the popover shadow; it fades in and rises 4px over 200ms with `--ease-out`, and fades out and drops 4px over 200ms with a plain ease-in when it goes; under Reduce Motion it only fades. It goes after 4 seconds unless the pointer is over it, when a newer shot replaces it, or when a click pastes it. Dragging it out is a file drag: Finder, any app and a chat pane that takes attachments all take it, the pane as the Chat section's image drop. A click attaches it to the focused agent's next message, if that agent takes attachments, and focuses its composer.
- **Recording:** the record circle (a filled dot) starts a screen recording of the simulator. On the click the circle grows left into a pill over 200ms with `--ease-out`, the row staying centered: the recording's time ("0:02", m:ss, 13px/500 `ink`, tabular, 12px from the pill's left edge, 6px before the button), then the button as a `danger` square on a `danger` wash at 18% (26% on hover, 34% pressed). The time reads 0:00 until the simulator records and stops counting on the stop click; a click on the button stops the recording. When the recording ends the pill shrinks back to the circle and its dot. Under Reduce Motion the pill changes at once. The stopped video leaves the screenshot's thumbnail, its first frame, which drags out as a file the same way. A click on it does nothing: no agent takes a video. A recording that ends by itself, as when the simulator shuts down, leaves its thumbnail too and the circle shows the dot again. A recording stops when Chord quits or crashes, so the file is complete.
- **Focus:** a click on the screen gives it the focus. While it has the focus every key goes to the simulator, and ⌘ keys stay the window's shortcuts.
- **Off:** with no screen, a centered meta line at 40% of the view's height: "The simulator is off." and under it, 12px below, a secondary "Run" button (tooltip "Run (⌘R)", disabled while a build starts) that runs as ⌘R does; with a device as the run destination "Runs go to <device>. Pick a simulator in Run Destination to run here." and no button; the simulator's own note while it starts or restarts, or, when Xcode's private frameworks do not load, their message and "Run (⌘R) opens DeviceHub instead." with the selected Xcode's simulator app, and under the line a secondary button "Open in <app>", DeviceHub or Simulator (the same app, 12px below the line) that opens it.

### Preview view

- **Header:** a row over the screen, right of the list, as tall as the list's filter row so the model button lines up with the field: 30px with 4px top and bottom, 6px right and 12px left padding. At its right end the model button: an `ink-muted` phone icon and the model name, "Newest iPhone" while the project has no pick (tooltip "Preview Device"); 24px tall, 8px side padding, Inset radius, `raised` on hover, `selected` when pressed. It opens a native menu of the simulator models as the run menu lists them, the project's pick with a check mark. While a rebuild runs, a 12px spinner and "Rebuilding" in meta text sit left of it; a hot reload shows nothing.
- **List:** a column left of the screen, 200px wide by default, with a 1px `line` right edge that drags it between 140px and 320px, as the side panel's edge, and never past half the view's width, so a narrowed side panel keeps room for the screen; the width lasts until the app quits. It holds the Files filter field (placeholder "Filter"), then the worktree's `#Preview`s as Files tree rows. The list runs the view's full height, so its edge meets the panel's top bar. A file reads as a folder row with a file icon and its relative path; its previews sit one level in, each with its name and no icon, an unnamed one by its index ("Preview 2"). Files keep path order. The shown preview's row takes `selected` and `ink`. ↑ and ↓ move the focus between rows; a focused preview row takes the selection at once and shows its preview once the keys rest for 150ms; → and ← open and close a file, as in the Files tree. A click shows a preview or opens and closes a file. The pick stays the worktree's, whatever file the editor shows. Typing filters on name and file and opens every file. No match reads "No matches"; a worktree without a `#Preview` reads "No previews in this worktree". The list reads again when the window takes the focus. Showing the view widens the side panel to 560px at least; the next view gets back the width the panel had before.
- **Screen:** the Simulator view's screen, bezel and input, without its button row.
- **Compile error:** the last frame stays, and a bar on `raised` with a 1px `line` top edge sits along the view's bottom: a 16px `danger` error icon, then the first error, `file:line` in 12px `danger` and the message in 12px `ink`, which truncates, its full text as the tooltip, 10px 12px padding. A click opens the file at that line. The bar goes when a build succeeds.
- **Off:** the Simulator view's centered meta line until the preview app first runs on the booted preview simulator: "Pick a preview." with no pick, "Starting the preview…" while the preview simulator boots, "Waiting for the Run build…" while ⌘R builds, "Building the preview…" while the preview builds, the failure's message when the build or launch fails, and "Two previews already run: <worktree>, <worktree>." when the limit holds. A rebuild after that keeps the screen.

### Bottom panel and Terminal

- **Bottom panel:** one row under the panes and the editor, left of the side panel, 240px tall by default, on `canvas` with a 1px `line` top edge. Dragging the top edge resizes it between 120px and 800px, and it leaves the panes 160px at least; dragging it below 60px hides the panel and every view in it; the edge has a divider's 9px grab area centered on the border and takes `line-strong` on hover and drag. Open state and height are saved per worktree. It shows the Terminal, and in a worktree with an Xcode project the Build log or the Console in a second column right of it (see Build and Run).
- **Terminal:** ⌘T shows or hides the worktree's terminal in the bottom panel and focuses it when it opens. One login shell per worktree starts on its first show and keeps running, with its scrollback, while the panel is hidden or another worktree shows; typing `exit` or deleting the worktree closes the panel, and the next ⌘T starts a new shell. A 28px header with a 1px `line` bottom rule: "Terminal" at 12px/500 in `ink-secondary`, `ink` while the terminal has the focus, and at the right end a 24px Hide icon button (tooltip "Show or Hide Terminal (⌘T)"), 4px from the edge. Under it xterm.js on `canvas` with 8px top, 2px bottom and 8px left padding, its rows held to the bottom edge: `ink` text and block cursor, `editor-selection` selection, the 16 ANSI colors from the `term-*` tokens (xterm's Tango palette in dark; GitHub's light terminal palette in light, so yellow and white stay readable on white), `ui-monospace` (SF Mono) at the terminal font size, 10,000 lines of scrollback (Settings › Appearance › Terminal, 9 to 24px, 12px by default). ⌘ keys run the app's shortcuts and copy and paste; every other key goes to the shell.
- **Terminal links:** a web URL in the terminal output opens in the browser with ⌘-click; a plain click does nothing.
- **Terminal find:** ⌘F while the terminal has the focus opens the chat's find bar (Chat › Find) under the header, at the terminal's top right. Matches take `editor-match` laid over `canvas` (xterm takes only opaque colors); the current one adds a 1px `accent-ink` border and scrolls into view. Esc closes the bar and returns the focus to the terminal.

### Editor

- Fills the work area left of the side panel while a file tab is open; the panes stay mounted behind it. `canvas` fill, Code text at the editor font size (Settings › Appearance › Editor, 9 to 24px, 12px by default; diffs take it too), Body line height, a line number gutter in `ink-disabled` (`ink-secondary` on the cursor's line), no ring on focus, `editor-selection` for the selection, `editor-match` on the matching bracket and the `syn-*` colors for keywords, strings, numbers, types and functions; comments are `ink-muted` italic. It saves 1s after the last edit and when its tab goes.
- **Editing:** brackets and quotes close as you type them. ⌘-click adds a cursor and ⌥-drag a column of them, the cursor's line takes `editor-line`, see-through so the selection shows under it, and the other copies of the selected text take `editor-match`. A fold gutter right of the line numbers shows a 12px `ink-muted` chevron on every block that folds (⌥⌘[ and ⌥⌘]); a folded block reads as a `raised` placeholder with the Inset radius.
- **Completion:** typing opens a list under the cursor of the file's own words and the language's keywords: `sheet`, a 1px `line` border, the Control radius, `--shadow-popover`, 4px padding, 240px tall at most. Rows are 24px in Code text, `ink-secondary`, 8px side padding, the Inset radius, no icons; the typed part takes 600 and the highlighted row `selected` and `ink`. ↑ and ↓ move the highlight, Return picks, Esc closes.
- **Find:** ⌘F in the editor opens the chat's find bar at the editor's top right. Matches take `editor-match`; the current one adds a 2px `accent-ink` underline and scrolls to the middle. A new query starts at the first match after the caret. Esc closes the bar, selects the current match and returns the focus to the editor.
- **Image view:** an image file (PNG, JPEG, GIF, WebP) opens in its file tab as the image, not the editor: centered on `canvas` with 24px padding, scaled down to fit and never up. It reads the file again within a second of a change on disk; a file gone from disk closes its tab.

### Status bar

A 26px strip across the window's full width, under the sidebar and the main area, on `canvas` with a 1px `line` top edge. Its items sit on the strip's center line. The 28px gear icon button that opens Settings (tooltip "Settings (⌘,)") sits in a 36px footer row at the sidebar's bottom, 8px in, under a 1px `line` divider. At the row's right end, 8px in, a question mark icon button (tooltip "Help") opens the Help sheet (see Sheets). Both hide with the sidebar; ⌘, and the app menu still open Settings. The strip's items sit at the right with 12px padding, in 11px `ink-muted`, tabular figures. The strip always shows. Subscription usage reads `42% 2h 48m  ·  59% 4d 6h`: each window's percentage and the time left to its reset in its two largest units, windows joined by a middle dot with two spaces each side, refreshed each minute. Before it, 8px away, a 32 × 4px bar (2px radius, `line-strong` track, `accent-ink` fill; above 70% `status-waiting`, above 85% `status-error`) shows the shortest window's percentage. The tooltip lists each window as `5h 42% (resets 14:30)`; a reset a day or more away carries its weekday. A window past its reset is hidden. In a worktree with an Xcode project the simulators button sits before the usage (see Build and Run).

### Build and Run

Every part shows only in a worktree with an Xcode project. A failure is `danger` text and the toolbar's Run and Stop icons take `success` and `danger`; the rest is neutral, and the status colors stay with the agents.

- **Toolbar:** the title bar row over the panes and the side panel, as in Xcode: 40px on `canvas` with a 1px `line` bottom edge, dragging the window between its controls, 4px left and 8px right padding, so Run's glyph starts 10px in, where a tab's icon and a lone plus start in a tab bar, and the bar's left edge reads the same when you switch between a worktree with an Xcode project and one without; with the sidebar hidden it starts after the sidebar toggle. From the left: Run, a `success` play triangle (tooltip "Run (⌘R)"), which builds and runs, in place of a running app too; Stop, a `danger` stop square (tooltip "Stop (⌘.)"), disabled with the `ink-disabled` glyph while nothing builds or runs; Test, the Tests icon (tooltip "Test"), disabled until Chord runs tests; all three icon buttons. Then, 6px after them, the scheme pill, as in Xcode: 28px, Control radius, no fill at rest, `raised` over the whole pill on hover, 12px/500 `ink`, two buttons with a 12px `ink-disabled` chevron between them. The scheme button (tooltip "Scheme and Build Configuration", 8px left and 4px right padding): an `ink-muted` scheme icon, the scheme's name, then the configuration at 400 in `ink-secondary` after an `ink-muted` "·". The name and configuration always show: the project's pick at once, else what a build uses once the listing came. A click opens the scheme menu under it. The destination button (tooltip "Run Destination", 4px left and 8px right padding): an `ink-muted` phone icon, the destination's name ("Newest iPhone" without a pick) and a simulator's iOS version at 400 in `ink-muted`. A click opens the run menu under it. Each button takes `selected` on hover and when pressed, its parts 6px apart. The toolbar's first 360px hold Run through the scheme pill, fewer in a narrow toolbar; a longer scheme, configuration or destination grows into the space after them, so the activity field keeps its place until the pill comes within 8px of it. Then, centered between those 360px and the view buttons, the activity field: 28px, 420px at most and 120px at least, `raised` fill, Control radius, 10px left padding, 12px text on one line that truncates, its full text as the tooltip: a lead at 500 in `ink`, then a detail in `ink-secondary` that carries its own lead-in. "Building Chord · Compiling ChordKit" (the scheme, then the last step the log names with its target: Compiling, Linking, Signing, Compiling assets of; Rust reports no file counts), "Building Chord · Waiting for another build", "Installing Chord on iPhone 17 Pro", "Launching Chord on …" and "Running Chord on …"; "Build succeeded · today at 10:32" with the lead in `success`; "Build failed · today at 10:32" with the lead in `danger` and, at the field's right end, the error count as a 22px button (a 12px warning icon and the count at 11px/500 tabular, `danger`, Inset radius, `selected` on hover, tooltip "Show Build Log") that shows the Build view; "Build stopped · …" and "Chord exited · …" with the lead at 400; "No build yet" at 400. An end's time reads as a message's time does, with "today at" before today's clock. While it builds, installs or launches, a 2px line runs along the field's bottom edge where a 30% `accent-ink` segment slides from left to right every 1.4s, easing in and out; the line shows that work goes on, not how far. Without the scheme's listing the lead drops the scheme's name. A build beside the running app speaks first. Then the side panel's view buttons at the right end. Nothing overlaps: the scheme button never shrinks and the activity field gives way first; under a 900px toolbar the iOS version hides, under 640px the scheme's name and configuration.
- **Run menu:** a native menu, as the worktree card's. Run (⌘R), Build (⌘B) and Stop (⌘., disabled while nothing builds or runs), a separator, then the run destinations: connected devices under a disabled "Devices" line, then the simulator models under a disabled line per iOS version ("iOS 27.0"), newest first, a separator before each group after the first, iPhones before iPads. The project's pick carries a check mark. Without a simulator the list is one disabled "No Simulators" line. A pick that is gone falls back to the first simulator, and a notice toast names both.
- **Scheme menu:** a native menu, as the run menu. The worktree's schemes, a separator, then its project's configurations under a disabled "Build Configuration" line. Each checks the project's pick, else the scheme a build without a pick uses and the configuration that scheme's Run action names. ⌘R, ⌘B and the agent's builds use the picks; the preview builds the default.
- **Product menu:** a menu bar menu after View, as Xcode's: Run (⌘R) and Build (⌘B), a separator, Clean Build Folder (⌘⇧K), a separator, Stop (⌘.). Each item is disabled while its command cannot run: outside a worktree with an Xcode project, and Stop while nothing builds or runs.
- **Run marker:** on the worktree's card, at the meta line's right end, as a session row's status sits, 8px from the git counts at least, 11px/500, tabular: "Building" while it compiles and "Running" in `accent-ink`, "Build succeeded" in `success` once the build is done (a run's too, while the app installs and launches) and "Build stopped" in `ink-muted` until the user goes to the worktree, or for 5 seconds on the open worktree's card, "Build failed" in `danger` until the worktree's next build starts. It fades in and rises 4px as the session row's status does. Every build of the worktree sets it, the agent's too.
- **Bottom panel views:** the log half: one column right of the Terminal that shows Build or Console, split from the Terminal by a 1px `line` rule. Dragging that rule resizes the log half, 480px wide by default, at least 200px, and it leaves the Terminal 200px at least; the rule has the bottom panel edge's grab area and hover. The width is saved per worktree. Each half shows on its own, and alone it takes the panel's width; the panel hides when neither shows. The log half has the Terminal's 28px header: two tabs, Build and Console, 2px apart, in the place of a name, then a Hide button at the right end that hides that half only. A tab follows the Tab rule at the header's size: 24px tall, 10px side padding, Control radius, 12px/500 `ink-secondary`, the Terminal's name color; hover and the shown view's tab take `raised`, and the text color stays. A click switches the view; the Terminal's header keeps its name at 12px/500 in `ink-secondary`, focused or not. Console has no clear button: no control in this document clears a view, and a build clears it. ⌘T shows or hides the Terminal; ⌘⇧Y shows or hides the log half. Both are saved per worktree.
- **Which view a run shows:** ⌘R and the play button open the side panel on the Simulator view when the run goes to a simulator. A build the user starts (⌘R, ⌘B, the play button) shows the log half on Build and leaves the Terminal as it is; when the app launches the half switches to Console, if it still shows Build; a failed build stays on Build. A build the agent started moves nothing; the run marker reports it.
- **Build view:** the build's output as Code text in `ink-secondary` on `canvas`, 8px padding, selectable, one scroll area that stays at the end while output arrives unless the reader scrolled up. Only the lines in view are mounted; the log keeps 10,000. After a failed build its errors come first in the scroll area, 30 at most, and the view scrolls to them: 26px rows with the Control radius, a 16px `danger` warning icon, then the file name and line ("LoginView.swift:42") in `ink-secondary`, then the message in `ink`, which truncates and holds the full text as its tooltip. Hover takes `raised`; a click opens the file at that line in the focused group, as the Files tree does. An error without a file shows its message only and takes no click. More than 30 end with "and 12 more" in meta text. Before the first build the view reads "No build yet" in meta text.
- **Console view:** the running app's output in the Build view's text style, cleared when a build starts, 10,000 lines. An OSLog error or fault line takes `danger`; Chord's own line ("Process exited with code 1") takes `ink-muted`. Until the app prints, the view reads "No app output yet" in meta text. ⌘⇧Y with neither column showing opens Console once the app has run, else Build.
- **Find:** ⌘F in the Build or Console view opens the chat's find bar under the header, as in the Terminal.
- **Simulators:** a text button at the status bar's right, 16px before the usage, shown while a simulator Chord created is booted, in any worktree: a 16px phone icon and the count in 11px `ink-muted`, tabular, 4px apart; 20px tall, 8px side padding, Inset radius, `raised` on hover, `selected` when pressed; tooltip "2 Simulators Running". A click opens a native menu: one disabled line per booted simulator ("feature-login · iPhone 17 Pro", a preview simulator "feature-login · Preview · iPhone 17 Pro"), a separator, then Shut Down Others, which keeps the open worktree's run and preview simulators and is disabled when no other is booted.
- **Build data:** a row in Settings › General under a "Storage" heading, shown once a worktree has an Xcode project: "Build Data", the description "DerivedData folders and simulators of your worktrees. Deleting a worktree deletes its share.", and the size at the right in `ink-secondary`, tabular ("12.4 GB"). No button: Chord removes the data of a worktree that is gone by itself.
- **Screenshots:** Settings › Simulators, shown once a worktree has an Xcode project, holds a group "Screenshots and recordings" above "Background services": a row "Save to" with the folder's path as its description, "Desktop" until the user chooses one, and a secondary "Choose…" button at the right that opens the folder dialog. Cancelling keeps the folder. Screenshots and recordings both go in it.
- **Simulator services:** Settings › Simulators, shown once a worktree has an Xcode project, holds one group "Background services" with a meta notice at its right ("Applies at the next boot · iOS 18.5 and newer"). A row per service group: its name, a meta description that wraps (what stops working while it is off, then "Frees about 265 MB."), and an On / Off segmented control, labeled "Keep <name> on", at the right. The description sits under the name, aligned to it. A group with several subgroups has a chevron before its name, and a click on its name or description opens the row, the chevron turning as a folder's does, to one row per subgroup under it, indented to the group's name, with the same parts: name, effect, an estimated memory and its own switch. The group's switch sets all its subgroups; it shows On when all are on, Off when all are off and neither selected when they differ, and the group's description then ends with "<n> of <m> on." A group with one function has no chevron and no subgroups; its name keeps the chevron's space, so every name lines up. App Store, Push & Media and Safari Sync & Web Services start On with all their subgroups, every other group Off.

### Chat

- **User message:** right-aligned, 85% wide at most, on a `bubble` card (Card radius, 10px 14px padding). A steer the agent has not taken in yet sits at the end of the transcript as a user message at 50% opacity (tooltip "Queued"); it turns opaque once the agent takes it in. The agent's message is Markdown on the canvas; the user's stays plain text. A user message taller than 12 lines folds to them, its last 2 lines fading out, with a "Show more" text button under the text (12px `ink-muted`, `ink-secondary` on hover, `selected` when pressed, Inset radius); open, it reads "Show less", and folding scrolls the bubble back into view. An open find bar unfolds every message and hides the button. Its images follow the text as 64px square thumbnails (Control radius, 1px `line` border, cropped to fill), 8px apart. A click opens the lightbox.
- **Message meta:** a 24px row 4px under each finished message, under the bubble's right edge for the user's, at the text's start for the agent's: a 24px copy icon button (tooltip "Copy Message"; it copies the message's text, the agent's as Markdown, and shows a check for 1.5s), then the message's time in 12px `ink-muted`, tabular: the clock today ("20:33"), "yesterday at 20:33", else the date ("May 3 at 20:33"), the year added outside the current one; its tooltip holds the full date. The row keeps its space and shows only while the pointer is over the message or focus is inside the row, fading over 120ms. A reply that never ended shows no time.
- **Lightbox:** the image at full size on the scrim, over everything (Control radius, sheet shadow); it fades in and rises like a sheet. A 28px close button sits at the top right, and with several images a previous and a next button at the left and right edges (disabled at the ends) and a "2 of 3" chip at the bottom; the buttons are 28px `canvas` squares with a 1px `line-strong` border and the popover shadow. ← and → step, Esc closes (the agent keeps running), a click outside the image closes.
- **Markdown:** blocks stack 4px apart, inside a quote too. Headings take the Heading row at every level. Lists indent 20px; a task shows a 14px `line-strong` box, with a check once done. A quote has a 2px `line-strong` rule on its left and `ink-secondary` text. A link is underlined in `ink-disabled`, in the text color (`ink` underline on hover), and takes the focus ring on keyboard focus; a click opens it in the default app, and only web and mail links open. A table has 1px `line` borders, a `panel` header row and scrolls sideways when wide. Inline code sits on a `raised` chip with the Inset radius.
- **Code block:** a `panel` card with a 1px `line` border and the Control radius. No language label. The copy icon button sits 4px from the top and right edges and shows only on hover or keyboard focus; it shows a check for 1.5s after it copied. The code scrolls sideways and never wraps.
- **Find:** ⌘F opens a bar floating at the transcript's top right (`--shadow-popover`, on `canvas` with a 1px `line-strong` border and the Control radius): a field, the position ("2 of 7", "No results"; meta, tabular), previous and next icon buttons (disabled without matches) and a close icon button. Return steps to the next match, Shift-Return to the previous, Esc or a click anywhere outside the bar closes it. Matches take an `editor-match` background; the current one adds a 2px `accent-ink` underline and scrolls to the middle.
- **Step group:** thoughts and tool calls between two messages, one row in meta text: a toggle with a chevron and what the group did by kind ("1 thought, 2 searches, edited 2 files, ran 4 commands": thoughts, files read, searches, folders listed, distinct files edited or written, commands run, other tool calls, each only when there is one, on one line that truncates with an ellipsis while the chevron, counts and time keep their place; Inset radius, `raised` and `ink-secondary` on hover, `selected` when pressed), folded when the run is past it. After the count, the lines the group's finished edits and writes added ("+43", `success`) and removed ("−5", `danger`), tabular, each only when not zero; a write counts its whole content as added. It stays folded while it runs, only the count growing; the toggle opens all steps. A call is its title (`ink-secondary`, `danger` when it failed) and its detail on one line. A call that ran 5 s or more, or failed, shows its run time at the line's right end ("55.4 s", "320 ms"), 11px `ink-muted`, tabular; the group's total sits at the toggle line's right end once it reaches 5 s. From a minute up a time reads in minutes ("9m 16s"). A call the run left unfinished (stopped with Esc) has an `ink-muted` title. Images the group's tools returned sit once in a row below the group's last line, folded or open: the same 64px thumbnails, 8px below it, in call order, opening the same lightbox. No images, no row.
- **Working:** a row of meta text right above the composer while the agent runs. It floats over the foot of the transcript, so a run's start resizes nothing; the transcript's bottom fade moves up to end at the row's top, and the text under the row hides; while it shows, the transcript ends that much lower. The row holds the `status-running` orbit, a verb in `ink-secondary` ending with "…" ("Cooking…", "Locking in…", "Hold my beer…", "Going feral…"), the time since the run started ("14s", "2m 5s") and "esc stops". The run's start time picks the verb, so it holds for the whole run.
- **Notice:** one line of `danger` meta text above the composer.
- **Compaction:** where the agent replaced the earlier messages with a summary, the transcript holds one row that reads as a step group's toggle line: the chevron and "Compacted 150K tokens" ("Compacted the context" when the agent gives no count), folded. The toggle opens the summary under it as Markdown at the transcript size in `ink-secondary`, indented 18px like a step. The messages before the summary are gone from the session, so the row is where the transcript starts.
- **Stopped:** a run the user stops with Esc ends in a left-aligned "Stopped" row of meta text in `ink-muted`, after the run's last item.
- **Sending** keeps the transcript where the reader left it; it follows the reply only when the reader sits at the end.
- **Subagent line:** when the harness has the subagents capability and an extension reports background runs, the lines sit in the transcript where the first run showed up, after the items that came before it, and stay there when later messages arrive or the runs end; a session's history puts them after its last item. They take no card: no fill, no border, the transcript's own inset, 4px between lines. Its lines read as the Working row does: meta text (12px, `ink-muted`, tabular), a `StatusMark` (orbit while it runs, `status-done` or `status-error` after) and, 8px after it, one joined text. A single run is one line, "worker · bash · 4s": name, tool and time joined with " · ", each part only when known, no leading separator. Two or more leaf rows (a chain, a fan-out) get a summary line first, in `ink-secondary`, "3 subagents · 12s" (the count of the leaf rows, the agents that hold no others, so a chain of three steps reads 3; then the time from the first start to now or to the last end), its mark the orbit while one runs, else `status-error` when one failed, else `status-done`; the rows follow, indented 18px per level once the summary shows (what a run holds sits one level deeper; without a summary the top rows sit flush), each "name · tool · time" with its own mark. A row's name is its label, or its agent when the label only repeats the step's key ("task-1"). A parallel run named after its children's agents ("scout, scout") gets no row of its own; its children take its level. The time ticks while a run runs and freezes at its length once it ended. A chain of single children that repeat one name (a run, its step, a nested summary of the same agent) is one row at any depth: it keeps the deepest tool, the earliest start, the deepest end and one state: running while any of them runs (a nested run can outlive its parent), else failed if any failed, else done; a running row has no end, so its time keeps ticking. A stop or a turn's end keeps the lines, because background runs outlive the turn; the lines stay through an automatic context compaction; the extension's own empty report on `/new` or `/resume`, and the agent's exit, clear them. With no run reported, or once the extension clears them, nothing shows. Chord has no control that stops a background run.
- **Composer:** a `sheet` card with a 1px `line` border, the Card radius, 10px 10px 8px padding and `--shadow-composer`. The draft is 14px at 1.625. Focus inside changes nothing. The text area starts one line tall, grows with the draft up to 224px, then scrolls inside; a new pane width or text size fits it to the draft again. While the agent is not ready the text area is inert and its placeholder takes `ink-disabled`. Pasted and attached images wait above the text as 48px thumbnails with an 18px remove button on their top-right corner; a thumbnail takes the transcript thumbnail's zoom cursor and `line-strong` hover border and opens the lightbox. Its bar holds the attach icon button (paperclip, tooltip "Attach Images") on the left, shown only when the harness has the attachments capability; after it the branch button (tooltip "Switch Branch"): a 28px picker button with the 16px branch icon and a 12px chevron in `ink-muted`, 6px apart around the worktree's branch in `ink-secondary` ("Detached HEAD" without one), at most 40% of the bar; while any agent in the worktree runs it takes `ink-disabled`, no click and the tooltip "Switch branches after the agent stops", and an open branch menu closes. Then, at the bar's right, when the harness has the models capability and has told its model, two plain text buttons side by side that read as one label: the current model's name in `ink-secondary` and, only when that model reasons, the thinking level capitalized in `ink-muted` (for example "Sonnet 5.5 Medium"): 28px tall, 8px padding on both sides, no gap, body size with no icon and no fill, `raised` on hover with the text one step brighter (`ink`, `ink-secondary`), `selected` when pressed, `ink-disabled` and inert while the agent is not ready (as the attach button), one line each. Nothing in the bar overlaps: as the bar narrows, the branch and model names ellipsize first, then the branch name and the context meter's percentage hide under 300px (the icons, the ring and their tooltips stay), the thinking level under 240px and the model name under 140px; the attach button and the send button never shrink. A click opens that picker as a menu above the composer (see Picker menu); a second click on the same button closes it, and a click on the other button swaps to its picker. Between the model label and the send button, when the harness has the context capability and has told its window, the context meter, set off from the label by a 1px `line-strong` divider 14px tall so it never reads as part of it: a 12px ring (2px stroke, `line-strong` track, the fill in `accent-ink`) and, 6px after it, the session's fill of the window as a percentage in 12px `ink-muted` meta text, tabular, 8px padding on both sides, with a tooltip of the tokens used and the window; from 50% the ring and the percentage take `status-waiting`, from 75% `status-error`, and the tooltip adds "nearly full". `/new` empties it (0%). The send button sits on the right: a 32px `accent` circle with a white up arrow, `selected` with an `ink-muted` arrow while the draft is empty. Its tooltip reads Send, or Steer while the agent runs and the harness has the steer capability; without it the button stays disabled until the run ends.
- **Dialog:** a dialog an agent's extension opens (select, confirm, input) takes the composer's place as the same `sheet` card: a Heading title, then the body, and the composer returns once it is answered. Esc cancels it. A select is a list of 30px rows in `ink-secondary`, 224px tall at most, scrolling inside; the highlighted row takes `selected` and `ink`, moves with ↑ and ↓ or the pointer, and Return or a click picks it. A select opens with the current option highlighted, if it has one, and marks it with a 16px `ink-muted` check at the row's right end. Every select starts with a Field (placeholder "Filter") that narrows the rows by what they contain, and reads "No matches" in meta text when none is left. A confirm shows its message in `ink-secondary` and a secondary "No" and a primary "Yes", right-aligned; an input shows a Field and "Cancel" and "Submit".
- **Slash menu:** while the draft is only a slash and a command name, a card of its own floats above the composer card, 6px clear of it and as wide: `sheet`, the composer card's own background, a 1px `line` border, the Card radius, `--shadow-popover`, 6px padding, 372px tall at most (ten rows), scrolling inside with the global scrollbar (see Scrollbars) and keeping the highlighted row in view. Its rows are 36px with 10px padding on both sides and no dividers, one line each: a 16px `ink-muted` outline icon for the command's kind (skill, prompt, extension or session; one per kind, none per command; a session command takes the tab's chat icon), then after an 8px gap its title in `ink` at the body size (the name without a `skill:` prefix, split on `-`, `_` and `:`, each word capitalized), then a one-line `ink-muted` meta description that ends in an ellipsis, then a right-aligned `ink-muted` meta label that never shrinks and sits 24px (the 8px gap plus 16px) after the description, even when it is ellipsized: "Local file" for a skill or prompt from a file, the package name for one from a package, "Extension" for an extension command, "Session" for a session command (Model, Thinking, New, Resume, Fork, Clone, Tree, Export), which runs when sent instead of going to the agent. The highlighted row takes `selected` with the Control radius and its description and label step up to `ink-secondary`. No chevrons. A command whose name equals the typed part comes first, then names that start with it; the typed part of the title, when it matches from the start of a word ignoring case and separators, takes 600, and nothing is marked when it does not. ↑ and ↓ or the pointer move the highlight; Tab or a click completes the name and a space; Return completes it too, unless the draft already is that name, then it sends. Esc or a click anywhere outside the list closes it until the draft changes. A harness that lists no commands shows no menu. A message that began as `/skill:name` shows that text in its bubble, never the skill's body.
- **Picker menu:** the model and thinking level pickers float above the composer card, which stays in place, as the slash menu does: the same card, position, size and scrolling. Neither picker has a Field; its list takes the focus. In the model picker, typing moves the highlight as a Mac menu does: to the first row with a word that starts with the typed letters, else the first that contains them. When the typed letters match nothing, the longest matching tail of them wins, so typing "fable" right after "opus" lands on Fable without deleting anything. The rows never narrow. A 1s pause starts a new name, and the highlight stays put when nothing matches. Then come the options as 36px rows in `ink` with 10px padding, the current one marked with the dialog's check. ↑ and ↓ or the pointer move the `selected` highlight, Return or a click picks. Esc or a click anywhere outside the card closes it without a pick. Replies and tool calls from the agent leave it open; only the agent's exit closes it. The branch button opens the branch menu the same way: the worktree's local branches, newest commit first, each row's meta line at its right end before the check slot (the age, as a session row's, or "In harbor-flaky · 5h" for a branch another worktree holds, which shows in `ink-disabled` and takes no highlight or pick), in 12px `ink-muted`, `ink-secondary` on the highlighted row. Up to four branches it moves the highlight by typing, as the model picker; from five it starts with a Field (placeholder "Filter") that takes the focus and narrows the rows, reading "No matches" when none is left. A pick switches the worktree to the branch; git's refusal shows in a failure toast. Right-click on a branch row opens Rename Branch and, for a branch no worktree or the main checkout holds, Delete Branch; either one closes the branch menu. Rename Branch opens a sheet with a Branch name field, the branch selected in it, and a primary Rename button; the field hint shows the typed name or, in `danger`, the live check's or git's reason, which disables Rename and keeps the sheet open. Delete Branch asks with a confirm sheet ("Delete Branch?", the destructive button "Delete"); when git calls the branch unmerged, a second one asks again ("Delete Unmerged Branch?") before it deletes with force.
- **Jump to latest:** once the reader scrolls 72px or more above the end of the transcript, a 28px `canvas` circle with a 1px `line-strong` border, the control shadow and an `ink-secondary` down arrow sits centered 4px above the Working row's place, whether the row shows or not. It shows no status: the Working row does. Hover takes `raised` and `ink`, pressed takes `selected`. A click scrolls smoothly to the end and resumes following the reply. It fades in and rises 8px from 90% scale over 200ms with `--ease-out`, fades out the same way once the reader is back at the end, and presses to 94% scale.
- **Empty state:** a centered title, one muted line and the primary button.
- **Starting:** while an agent starts in an empty pane, a 14px `ink-muted` spinner sits centered in the transcript ("Starting the agent" for VoiceOver).

### Sheets

- A `sheet` card centered over the window, which blurs 12px behind it and keeps its brightness. A click outside the sheet cancels. While a sheet is open the window behind it is inert: Tab and clicks stay in the sheet.
- **Form sheet:** a header row with the title on the left and a close icon button (X) on the right that cancels. The fields follow, then a footer with the primary button on the right. Return triggers the primary button, Escape cancels.
- **Focus after a sheet:** when a sheet or Settings closes, the focus goes back to the control that held it before, unless the sheet's action moved it or that control is gone.
- **Help sheet:** 440px wide, 20px between blocks. A form sheet header titled "Help", then three blocks, each under a field label: Basics (project, worktree, session: the name at 500, a line in `ink-secondary` under it), Statuses (each `StatusMark` with its word in the status color, the line under it) and Shortcuts (a command title on the left, its keys in `ink-muted` on the right; a command with no key is left out). A secondary "All Shortcuts" button in the footer opens Settings on Shortcuts.
- **Confirm sheet:** title, one centered `ink-secondary` line (`.sheet-description`) and an optional detail in meta text, 4px apart as one block; 20px below it two equal 32px buttons: secondary "Cancel" and the destructive or primary action. Focus starts on the action, Escape cancels, and a long name or path wraps inside the sheet. The detail is selectable text.

### Settings

- A panel centered over the window, which blurs behind it as behind a sheet: `canvas`, Card radius, sheet shadow, 960 × 760px at most and 28px clear of the window edges. It rises in like a sheet. Esc, the close icon button or a click outside it closes it; a sheet opened over it closes first. The window behind it is inert.
- **Section list:** 220px on the left, `panel` with a 1px `line` right edge, 12px 8px padding, rows 2px apart: General, Appearance, Shortcuts, and Simulators once a worktree has an Xcode project (a phone icon). A row is 30px, 10px padding, Control radius, a 16px `ink-muted` icon and the name in `ink-secondary` 8px after it. Hover takes `raised` and `ink` over 120ms, pressed takes `selected`; the shown section takes `selected`, `ink` at 500 and an `ink-secondary` icon. The focus ring sits inside the row. It opens on General.
- **Header:** 48px with a 1px `line` bottom rule, 28px left padding: "Settings / General" at Heading size, "Settings" in `ink-muted`, the slash at 400 in `ink-disabled`, the section in `ink`. A close icon button sits at the right.
- **Content:** scrolls inside; 640px at most, centered, 28px side padding. Each group takes a Heading (28px above, 8px below, a status notice in meta text at its right), then a `panel` card with a 1px `line` border and the Card radius.
- **Row:** 44px at least, 10px 16px padding, a 1px `line` rule above every row but the first. The name in body `ink` on the left, a one-line meta description in `ink-muted` under it (an error takes `danger`; Settings › Simulators rows wrap it), the control on the right.

### Toast

One surface in the title bar row, 32px tall at least, 560px wide at most. Its text is selectable; a 24px close icon button (tooltip "Dismiss") sits 4px from its right end. A notice shows text only and closes after 4 seconds. A failure shows a `danger` warning icon before the text and stays until its close button, so an install step can be read and copied.

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
- **Don't** end a label, title or status with "…" or "...". The chat's working verb is the one exception.
