---
version: alpha
name: chord-design
description: "A macOS workspace for coding agents. One window holds a white canvas; the UI stays monochrome (near-black ink, warm off-white chrome, soft grays). The toolbar lives in the title bar as floating pill groups next to the traffic lights. Toasts and action bars float as pills, sheets and panels are large rounded cream cards over a light gray scrim. One button language runs through the app: black pill for the primary action, white or light gray pill for everything else. Dark appearance inverts the same system."

platform: "macOS 14+, single main window with a full-size content view and a transparent title bar. Sizes are in points."

colors:
  ink: "#111111"
  ink-secondary: "#5C5C5C"
  ink-muted: "#8E8E8E"
  ink-disabled: "#B5B5B5"
  canvas: "#FFFFFF"
  chrome: "#FAF9F7"
  surface-cream: "#FBFAF8"
  surface-muted: "#F2F1EF"
  surface-pressed: "#EBEAE8"
  skeleton: "#F1F0EE"
  hairline: "#ECEBE9"
  hairline-strong: "#DEDDDB"
  focus-ring: "rgba(17, 17, 17, 0.35)"
  scrim-light: "rgba(190, 190, 190, 0.55)"
  scrim-dark: "rgba(0, 0, 0, 0.55)"
  primary: "#111111"
  on-primary: "#FFFFFF"
  danger: "#C0392B"
  success: "#3E8E41"
  notification-dot: "#3B82F6"
  dark-canvas: "#141414"
  dark-chrome: "#1F1F1F"
  dark-surface: "#242424"
  dark-hairline: "#2E2E2E"
  dark-ink: "#F5F5F5"
  dark-primary: "#FFFFFF"
  dark-on-primary: "#111111"

typography:
  fontFamily: "SF Pro (system font); SF Pro Display above 20pt"
  page-title:
    fontSize: 22pt
    fontWeight: 400
    lineHeight: 1.2
    letterSpacing: -0.3pt
  section-title:
    fontSize: 15pt
    fontWeight: 500
    lineHeight: 1.3
  sheet-title:
    fontSize: 14pt
    fontWeight: 500
    lineHeight: 1.3
  body:
    fontSize: 13pt
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontSize: 13pt
    fontWeight: 500
    lineHeight: 1.3
  meta:
    fontSize: 11pt
    fontWeight: 400
    lineHeight: 1.35
    color: "{colors.ink-muted}"
  micro:
    fontSize: 10pt
    fontWeight: 400
    lineHeight: 1.3

rounded:
  input: 8pt
  card: 14pt
  sheet: 20pt
  pill: 9999pt

spacing:
  xxs: 4pt
  xs: 8pt
  sm: 12pt
  md: 16pt
  lg: 24pt
  xl: 32pt
  xxl: 48pt
  window-margin: 20pt
  titlebar-height: 52pt
  traffic-light-inset: 80pt

window:
  minSize: 900 x 600
  defaultSize: 1280 x 820
  titleBar: transparent, full-size content view, no visible title
  toolbar: pill groups placed in the title bar row, vertically centered with the traffic lights

elevation:
  chrome-pill: "0 1px 2px rgba(0,0,0,0.04), inset 0 0 0 1px {colors.hairline}"
  card: "0 2px 12px rgba(0,0,0,0.06)"
  popover: "0 8px 28px rgba(0,0,0,0.10)"
  sheet: "0 16px 48px rgba(0,0,0,0.12)"

components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.label}"
    rounded: "{rounded.pill}"
    height: 28pt
    padding: 0 14pt
  button-primary-large:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.pill}"
    height: 40pt
    width: 100%
  button-primary-disabled:
    backgroundColor: "{colors.surface-muted}"
    textColor: "{colors.ink-disabled}"
  button-secondary:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.ink}"
    border: "1px solid {colors.hairline}"
    shadow: "{elevation.chrome-pill}"
    rounded: "{rounded.pill}"
  button-tertiary:
    backgroundColor: "{colors.surface-muted}"
    textColor: "{colors.ink}"
    rounded: "{rounded.pill}"
  button-destructive:
    backgroundColor: "{colors.surface-muted}"
    textColor: "{colors.danger}"
    rounded: "{rounded.pill}"
  icon-button:
    backgroundColor: "{colors.canvas}"
    border: "1px solid {colors.hairline}"
    rounded: "{rounded.pill}"
    size: 28pt
  input:
    backgroundColor: "{colors.surface-cream}"
    border: "1px solid {colors.hairline}"
    rounded: "{rounded.input}"
    height: 32pt
    padding: 0 12pt
  toolbar-pill:
    backgroundColor: "{colors.chrome}"
    shadow: "{elevation.chrome-pill}"
    rounded: "{rounded.pill}"
    height: 36pt
  sheet:
    backgroundColor: "{colors.canvas}"
    rounded: "{rounded.sheet}"
    shadow: "{elevation.sheet}"
    scrim: "{colors.scrim-light}"
  popover:
    backgroundColor: "{colors.surface-cream}"
    rounded: "{rounded.card}"
    shadow: "{elevation.popover}"
  toast:
    backgroundColor: "{colors.canvas}"
    rounded: "{rounded.pill}"
    shadow: "{elevation.popover}"
    position: bottom-center of the window
  segmented:
    track: "{colors.canvas} with 1px {colors.hairline}"
    selected: "{colors.primary} pill, {colors.on-primary} text"
---

## Overview

Chord is a native macOS workspace for coding agents: one git worktree per task, each with its own agent chats. The interface uses near-black, white and a family of warm off-whites, with no brand accent color. The only hues are the red of destructive text, the green of a success message and a small blue notification dot.

Toolbars, menus, toasts and action bars float above the canvas as rounded pills and cards, so the chrome never boxes the content in.

Every color, shadow and font lives in `src/ui/tokens.css` as a CSS variable named after the keys above (`--ink`, `--canvas`, `--shadow-sheet`). Components use the variables and hold no raw color value. Add every new color token to both the light `:root` block and the dark block.

The app keeps the platform conventions a Mac user expects: traffic lights, a full menu bar, keyboard shortcuts for every command, a separate Settings window (⌘,), drag and drop from Finder and system appearance (light / dark).

## Colors

### Ink

- `ink #111111` for titles, labels, primary buttons and selected states.
- `ink-secondary #5C5C5C` for descriptions in sheets.
- `ink-muted #8E8E8E` for meta lines, timestamps and placeholders.
- `ink-disabled #B5B5B5` for disabled button labels.

### Surfaces

The canvas is pure white and runs under the transparent title bar. The chrome around it runs warm, a hair toward beige:

- `chrome #FAF9F7`: toolbar pill groups.
- `surface-cream #FBFAF8`: popovers, menus, panels, the Settings sidebar, input fills.
- `surface-muted #F2F1EF`: tertiary pill buttons, selected sidebar row, selected tab, disabled primary buttons.
- `skeleton #F1F0EE`: loading placeholders.

Skip system vibrancy materials on the canvas and toolbar. Flat warm fills keep the monochrome look stable over colorful wallpapers.

### Semantic

- Danger `#C0392B`: text only. Destructive menu items, the destructive confirm button (red label on a gray pill), inline errors.
- Success `#3E8E41`: inline text only.
- Notification dot `#3B82F6`: a 5pt dot on the tab or icon that holds something unseen. Mirror it with a Dock badge.
- Focus ring: a 2pt `focus-ring` outline outside the control's own radius. Keyboard focus must stay visible on every control.
- Text fields and selects (`.input`) get no outline. Focus changes only the border color to `focus-ring`, with no transition, so clicking between the Project and Branch name fields never flashes a ring.

Keep large areas free of semantic color. Confirmation uses a black circle with a white check.

### Dark appearance

The app follows the system appearance by default; View › Appearance overrides it (Light / Dark / System). Dark inverts the roles:

- Canvas `#141414`, toolbar pills `#1F1F1F`, cards and sheets `#242424`.
- Text `#F5F5F5`, muted text about `#8A8A8A`.
- The primary button flips to a white pill with black text.
- Selected chips and tabs flip to a white pill.
- Sheets use `scrim-dark`.

## Typography

### Font family

SF Pro, the system font. Use SF Pro Display for sizes above 20pt and SF Pro Text below; the system switches automatically when you ask for the system font.

### Hierarchy

| Role | Size | Weight | Where |
|---|---|---|---|
| Page title | 22pt | 400 | The title of the open view |
| Section title | 15pt | 500 | Section headings inside a view |
| Sheet title | 14pt | 500 | Sheet and dialog titles |
| Body | 13pt | 400 | Descriptions, form values |
| Label | 13pt | 500 | Buttons, tabs, menu items, chips |
| Meta | 11pt | 400, muted | Counts, timestamps, helper text |
| Micro | 10pt | 400 | Sublabels under icon buttons |

### Principles

- Titles stay at regular weight (400). Weight 500 marks interactive labels and sheet titles. Bold barely appears.
- Meta lines join facts with a middle dot: "3 chats · 2 hours ago".
- Counts sit in small gray number badges next to tab labels.
- Menu bar items and buttons use title case per the macOS guidelines ("New Task…"). In-canvas copy stays short and sentence case.

## Layout

### Window

- One main window, default 1280 × 820pt, minimum 900 × 600pt. The app restores size, position and the last open view on launch.
- Transparent title bar with full-size content: the white canvas runs to the top edge and the traffic lights sit on it.
- Dragging any empty part of the toolbar row moves the window; double-clicking it zooms.

### Toolbar

Floating pill groups in the title bar row, 36pt tall, vertically centered with the traffic lights. No full-width bar. The first group starts after the traffic lights (80pt inset). The active tab in a tab pill gets ink color and weight 500; inactive tabs are muted.

In full screen the toolbar stays; the traffic lights hide and the first group moves to the window margin.

### Text selection

Text is not selectable by default. Only content someone copies or edits is: chat messages, code and diffs, terminal output, file paths and error details, and the text in inputs. Chrome is never selectable: labels, titles, buttons, rows, tabs, toasts, menus, tooltips, empty states. Mark a selectable region with `data-selectable`; `src/ui/tokens.css` sets `user-select: none` on `body` and turns it back on for inputs and `[data-selectable]`. Set both `-webkit-user-select` and `user-select`, because the macOS web view ignores the unprefixed one.

### Whitespace

Content runs dense; the chrome stays airy. Sheets use 20 to 28pt padding. Centered views (empty states) leave large empty margins and keep content in a narrow column.

## Elevation & Depth

- Toolbar pills: near-flat, a hairline ring and a 1 to 2pt shadow.
- Cards: a soft 12pt shadow and a 14pt radius.
- Popovers and menus: a 28pt soft shadow on cream.
- Sheets: large soft shadow. In light appearance the scrim turns the window a desaturated light gray instead of darkening it.
- Native context menus and the menu bar keep the system look. Custom popovers use the cream style above.

## Shapes

| Element | Radius |
|---|---|
| Inputs | 8pt |
| Cards, popovers | 14pt |
| Sheets, Settings panes | 20pt |
| Buttons, chips, tabs, toasts, search, toolbar groups, action bars | full pill |
| Avatars, icon buttons | circle |

## Components

### Buttons

- Primary: black pill, white 13pt/500 label. Sizes: regular 28pt in the toolbar and inline, and large 40pt full-width in forms and sheet footers.
- The primary button of a sheet is the default button: Return triggers it. Escape triggers "Cancel".
- Disabled primary: `surface-muted` fill, `ink-disabled` label. Forms keep the button disabled until input is valid.
- Secondary: white pill with hairline ring ("Cancel").
- Tertiary: gray pill.
- Destructive confirm: gray pill with red label, paired with a white "Cancel".
- Icon buttons: 28pt white circles with hairline ring (back arrow, more "…", sort, filter). Each has a tooltip with its shortcut.
- Pressed: fill steps to `surface-pressed` (or 85% black for primary).
- Loading: the label swaps for a spinner inside the same pill.

### Inputs

- 32pt tall, 8pt radius, cream fill, hairline border. Labels sit above in 11pt muted text.
- Inline helper and error text sits under the field in red or green.
- Toggle switch: pill track, gray off, black on.
- Pop-up buttons: full-width cream field with chevron; the open list is a cream popover with 32pt rows.

### Segmented controls and tabs

- Segmented control: a hairline pill track with a black selected pill.
- Tabs: a hairline pill track, selected tab in `surface-muted`, count badges inline. ⌘1, ⌘2, ⌘3 switch them.
- Panel tabs use a 2pt black underline instead of a pill.

### Chips

Text only; the selected chip becomes a black pill.

### Sheets

- Sheets attach to the main window and appear centered over the scrim as white cards, 20pt radius, 20 to 28pt padding.
- Header patterns: centered title with a text "Cancel" left and an optional primary pill right, or small muted title left with an X right.
- Confirm dialogs: title, one muted line, two equal 40pt pills side by side (Cancel / destructive or primary); the muted line is `.sheet-description`, centered, with any detail beneath it as meta text.
- Success dialogs: black check circle, title, muted line, full-width "Done".

### Settings window

- Opens from App menu › Settings… (⌘,) as its own window, not a sheet. Fixed width 720pt, height fits the pane.
- Two panes: left sidebar on cream with icon + label rows, selected row in `surface-muted`. Right pane white with a 15pt title and grouped fields.
- Changes apply when the user makes them, per macOS convention. Text fields commit on Return or when focus leaves; no Save button.

### Popovers and menus

- Custom popovers: cream, 14pt radius, 32pt rows, label left, line icon right, destructive rows in red. They anchor to their toolbar button with no arrow.
- A popover with a list puts a search field on top; typing filters the list immediately.

### Menu bar

Every action in the app also lives in the menu bar with a shortcut. Edit, Window and Help keep the system defaults; Go holds Back (⌘[) and Forward (⌘]).

### Toasts

Bottom-center floating pill inside the window, white, soft shadow, 13pt label, dismissed after 4 seconds or on click. Three variants:

1. Label + black check circle ("Changes saved").
2. Label + black "View" pill.
3. Label only (errors and notices).

Toasts carry no warning or exclamation icon. Errors state the cause in text. A two-line variant adds a muted sublabel. Events that happen while the app sits in the background go to Notification Center instead.

### Floating action bar

Multi-selection puts a bottom-center pill bar on screen: the selection count, the actions, then "Done". Selected items get a 2pt black outline with a white inset gap. Delete removes the selection after a confirm sheet; Escape exits the mode.

### Empty and loading states

- Empty: a small icon, a 14pt title, a muted line and a gray pill CTA.
- Drop target: while the user drags files over the window, the canvas shows a 2pt dashed ink outline inset 12pt with a short label.
- Loading: skeleton blocks in `skeleton` gray with the same radii as the content they stand for.

### Icons

SF Symbols at regular weight, 15 to 17pt, in ink or muted gray.

## Do's and Don'ts

### Do

- Keep every UI element in ink, white or warm gray.
- Use the black pill for the single primary action per view; make it the default button in sheets.
- Float chrome as separate pills and cards above the canvas.
- Give every command a menu bar item and a keyboard shortcut.
- Use the light gray scrim for everyday sheets.
- State outcomes in a toast with a check or a "View" action.

### Don't

- Don't add a brand accent color to buttons, links or selection, and don't inherit the system accent color for them.
- Don't fill areas with red or green; keep semantic color in text and small icons.
- Don't use a standard opaque toolbar or bordered panels around the content.
- Don't mix radii: inputs stay at 8pt, cards at 14pt, actions stay pill.
- Don't use bold weights for headings.
- Don't put a Save button in the Settings window.
- Don't make chrome text selectable.
- Don't put a warning or exclamation icon on a toast, popup, sheet or alert. Text only.

## Window Resizing

- Below 1000pt the toolbar pills shrink first, then a tab pill collapses to the active tab with a chevron menu.
- Sheets cap at 80% of the window height and scroll inside.

## Iteration Guide

1. Start with tokens: `canvas`, `chrome`, `surface-cream`, `surface-muted`, `ink`, `ink-muted`, `hairline`, plus their dark counterparts.
2. Build the window shell: transparent title bar, toolbar pill groups beside the traffic lights, menu bar with shortcuts.
3. Build the pill button set (primary, secondary, tertiary, destructive, icon, disabled, loading).
4. Add sheet, popover and toast shells with the elevation tokens, then the Settings window.

## Known Gaps

- Motion: no timing or easing defined yet. Sheets and popovers should match system animation speeds and respect Reduce Motion.
- Accessibility beyond focus rings (VoiceOver labels, Increase Contrast variants) still needs a pass.
