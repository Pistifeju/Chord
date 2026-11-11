---
name: polish-ui
description: Run impeccable layout, typeset and polish on a Chord UI target in one pass, then check it for drift from DESIGN.md and its neighbors.
disable-model-invocation: true
---

One refinement pass over a UI target: three impeccable commands in a fixed order, then a Chord drift sweep. The pass refines the existing look. A finding that needs a redesign goes in the report, not in the diff.

## 1. Pin the target

- The user named files, a folder or a feature (`sidebar`, `worktrees`): that is the target.
- Nothing named: take the `.tsx` and `.css` files under `src/` from `git diff HEAD --name-only` plus `git ls-files --others --exclude-standard`.
- Neither gives a file: ask the user for a target and stop.

Done when: you hold a list of files and the feature folders they belong to.

## 2. Load context once

1. Read `.agents/skills/impeccable/SKILL.md` and follow its Setup: run `impeccable context --target <target>` once and read `reference/craft-floor.md` before the first edit.
2. Read `DESIGN.md`, `src/ui/tokens.css`, `src/ui/controls.css`.
3. Read one neighbor for each target file: a sibling component in another feature folder with the same role (list row, pill button, sheet, popover, empty state). The neighbor sets the bar for consistency.

Chord overrides to impeccable:

- Evidence comes from source: files, selectors, values. Skip screenshots, the dev server and the app window; the user checks the rendered result.
- The surface mode is Operate.
- Run each command's assessment yourself, in order. Use sub-agents only when the user asked for them.

## 3. Run the three commands

Run them in this order. Each one reads its reference in full, assesses, then edits before the next one starts. This skill never calls `impeccable detect` itself: layout and typeset run the scoped scan their reference describes, and polish runs none.

1. **layout**: `reference/layout.md`. Grouping, rhythm, density, window resizing (DESIGN.md › Window Resizing).
2. **typeset**: `reference/typeset.md`. Every text role maps to one row of DESIGN.md › Typography › Hierarchy.
3. **polish**: `reference/polish.md`. States, interaction, copy, cleanup. Skip its rendered-viewport checks per the overrides above.

Done when: each command has its assessment answered with file and selector evidence, and the findings of the scans layout and typeset ran are fixed or listed as intentional exceptions.

## 4. Drift sweep

Grep the target, and compare each hit with the neighbor from step 2 and the DESIGN.md section in brackets. Fix every mismatch.

- **Values**: raw hex, `rgb()` or `hsl()` outside `tokens.css`; a shadow, easing or duration written inline instead of its token. [Colors, Elevation & Depth]
- **Type**: every `font-size` and `font-weight` matches a Hierarchy row; the same role carries the same size and weight in every feature. [Typography]
- **Shape**: every `border-radius` matches the Shapes table; control heights match Buttons and Inputs. [Shapes, Components]
- **Color use**: semantic red or green stays in text and small icons; no accent color on buttons, links or selection; one black pill per view. [Do's and Don'ts]
- **States**: hover, pressed, focus ring, disabled and loading exist and match `controls.css`; empty and loading states follow their section. [Buttons, Empty and loading states]
- **Copy**: title case on buttons and menu items, sentence case in the canvas, meta lines joined with ` · `, an ellipsis `…` on commands that open a sheet. [Typography › Principles]
- **Shared pieces**: a local style that duplicates a class in `controls.css` or a component in `src/ui/` gets replaced with the shared one.

A value with no DESIGN.md rule and no neighbor to copy: leave it, and list it in the report as a DESIGN.md gap.

Done when: every bullet above was grepped on every target file.

## 5. Finish

1. Read the full diff and remove churn the pass created: unused classes, orphaned selectors, duplicate values.
2. Run `npm run check`. Fix failures it reports in files you touched.
3. Report, short:
   - what changed, grouped by command and sweep;
   - the intentional exceptions and DESIGN.md gaps;
   - redesign-sized findings you left alone.
