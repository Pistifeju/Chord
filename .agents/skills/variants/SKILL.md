---
name: variants
description: Build four versions of a UI request on one page from Chord's own tokens and components, open it, and rework a version on feedback. Asks no questions.
disable-model-invocation: true
---

Four versions of one UI request on one page the user compares them on. Each version looks different and acts different, and all four read as Chord. You ask nothing: decide every open point from the code, and list each decision as an assumption in the report.

## 1. Pin the request

- The request names the thing: a button, a row, a menu, an empty state, a sheet, a view.
- The spot is the component that will render it. The request names no spot: run `npm run map`, pick the feature folder whose job matches, and take the component the thing belongs in.
- The slug is the thing in two or three words, kebab case (`archive-button`).

Done when: you hold the thing, the spot's file and component, and the slug.

## 2. Load Chord's design

1. Read `.agents/skills/impeccable/SKILL.md` and follow its Setup: run `impeccable context --target <spot file>` once.
2. Read `DESIGN.md`, the `design/` file of the thing and of the spot, `src/ui/tokens.css`, `src/ui/controls.css`, and the spot's `.tsx` and `.css`.
3. Find every instance of the same kind of thing in the app: `rg` its class in `controls.css` and its component in `src/ui/`, then read the markup and CSS of each hit in the spot's view and in two other features. For a button that is every button in the spot's view, plus the same role elsewhere.
4. Write the **identity lock**: one sentence on what the spot shows today (surface token, type sizes, radius, spacing, how its controls behave), then the **kit**: the classes, `src/ui/` components, icons and tokens the versions will draw from.

Chord overrides to impeccable:

- Evidence comes from source: files, selectors, values. The user looks at the page; you take no screenshot.
- The surface mode is Operate.
- Do the work yourself, with no sub-agents and no `impeccable live` or `generate`.

Done when: the identity lock and the kit are written, and every kit entry names a real class, component or token.

## 3. Plan four versions

Each version commits to one **look move** and one **act move**.

- **Look moves**: hierarchy (what the eye lands on first), layout (stacked, side by side, inline, overlay), density, structure (merge, split, reveal on demand), which neutral step carries the surface, icon against label.
- **Act moves**: the trigger (click, hover reveal, keyboard first, drag), where the result shows (inline, popover, sheet, toast), the commit (at once, confirm, undo), feedback and motion from DESIGN.md › Motion.

Rules:

- Four different look moves. Four different act moves; a thing with no behavior of its own (a label, a still empty state) may share one.
- Every version stays inside the identity lock and DESIGN.md: its tokens, the Shapes table, the Named Rules, the Do's and Don'ts. Rework a version that needs a rule bent.
- Order them by distance: version 1 sits closest to what Chord does today, version 4 goes as far as the rules allow.

Done when: you hold one line per version (name, look move, act move, kit pieces), and no line could describe another version.

## 4. Build the page

The page is `.impeccable/mockups/<slug>/`, a folder git ignores. `src/` stays untouched.

1. `cp -R .agents/skills/variants/page .impeccable/mockups/<slug>`
2. Read `.agents/skills/impeccable/reference/craft-floor.md`.
3. In `main.tsx`, set `request`, fill the `versions` list from your plan, and write one component per version:
   - Import the kit: `src/ui/` components, icons, and the CSS file of each feature the thing borrows classes from.
   - A feature component that reads a store or calls IPC cannot run on the page. Copy its markup and class names into the version, and feed it fixture data.
   - Fixture data is real: the typical case and the longest one (a 40-character branch name, a count of 0 and of 1,000).
   - Every state the thing has works on the page: hover, pressed, focus ring, disabled, loading, open and closed, with the motion DESIGN.md gives it.
4. New CSS goes in the page's `variants.css` under the frame rules, one prefix per version (`.v1-` to `.v4-`), every value a token. Set `.variants-stage` to the width and the surface token of the spot.

Done when: `npx tsc --noEmit -p .impeccable/mockups/<slug>` exits 0, and `rg -n '#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(' .impeccable/mockups/<slug>/variants.css` prints nothing.

## 5. Open the page

The page's URL is `http://localhost:1421/.impeccable/mockups/<slug>/`. A server of its own serves it, so the app keeps port 1420.

1. `curl -sf -o /dev/null http://localhost:1421/` fails: start the server and wait until the same `curl` succeeds.

   ```sh
   nohup npx vite --config .agents/skills/variants/vite.config.ts > "${TMPDIR:-/tmp}/chord-variants.log" 2>&1 &
   ```

2. `open <URL>`: every time you create a page, and every time step 1 had to start the server. In every other case the tab the user has open reloads itself on each save, so leave it alone.

Done when: `curl -s -o /dev/null -w '%{http_code}' <URL>main.tsx` prints `200`.

## 6. Report

Short, in this order:

1. The version you would pick, with the reason in one line.
2. Each version: number, name, look move, act move, the trade-off in one line.
3. The assumptions you made instead of asking.
4. One line: "Reply with a number and what to change, or `use <number>`."

## 7. Rework on feedback

The user names a version and what to change ("1, but the label is too heavy", "3 with 2's hover").

- Edit that version in the same page: same slug, same files. Change what the feedback names; the other versions stay as they are.
- Feedback that turns down all four: plan four new ones (step 3) and replace the old ones in the same page.
- Unclear feedback gets its likeliest reading, named in your reply. You still ask nothing.
- Run step 4's checks, then step 5. Reply with one line per changed version.

Repeat on every round of feedback.

## 8. Build the pick in

Runs when the user says `use <number>`.

1. Build that version into the spot under `AGENTS.md`: its CSS moves to the feature's CSS file, and each rule it adds goes into the matching `design/` file.
2. Delete `.impeccable/mockups/<slug>/`. No page folder left in `.impeccable/mockups/`: stop the server with `lsof -ti tcp:1421 | xargs kill`.
3. Run `npm run check`.
