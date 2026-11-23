# Chat

- **User message:** right-aligned, 85% wide at most, on a `bubble` card (Card radius, 10px 14px padding).
  A steer the agent has not taken in yet sits at the end of the transcript as a user message at 50% opacity (tooltip "Queued"); it turns opaque once the agent takes it in.
  A message sent while the chat still loads waits the same way and goes out when the agent takes messages; until then the send button stays disabled.
  The agent's message is Markdown on the canvas; the user's stays plain text.
  A user message taller than 12 lines folds to them, its last 2 lines fading out, with a "Show more" text button under the text (12px `ink-muted`, `ink-secondary` on hover, `selected` when pressed, Inset radius); open, it reads "Show less", and folding scrolls the bubble back into view.
  An open find bar unfolds every message and hides the button.
  Its images follow the text as 64px square thumbnails (Control radius, 1px `line` border, cropped to fill), 8px apart.
  A click opens the lightbox.
- **Message meta:** a 24px row 4px under each finished message, under the bubble's right edge for the user's, at the text's start for the agent's: a 24px copy icon button (tooltip "Copy Message"; it copies the message's text, the agent's as Markdown, and shows a check for 1.5s), then the message's time in 12px `ink-muted`, tabular: the clock today ("20:33"), "yesterday at 20:33", else the date ("May 3 at 20:33"), the year added outside the current one; its tooltip holds the full date.
  The row keeps its space and shows only while the pointer is over the message or focus is inside the row, fading over 120ms.
  A reply that never ended shows no time.
- **Lightbox:** the image at full size on the scrim, over everything (Control radius, sheet shadow); it fades in and rises like a sheet.
  A 28px close button sits at the top right, and with several images a previous and a next button at the left and right edges (disabled at the ends) and a "2 of 3" chip at the bottom; the buttons are 28px `canvas` squares with a 1px `line-strong` border and the popover shadow.
  ← and → step, Esc closes (the agent keeps running), a click outside the image closes.
- **Markdown:** blocks stack 4px apart, inside a quote too.
  Headings take the Heading row at every level.
  Lists indent 20px; a task shows a 14px `line-strong` box, with a check once done.
  A quote has a 2px `line-strong` rule on its left and `ink-secondary` text.
  A link is underlined in `ink-disabled`, in the text color (`ink` underline on hover), and takes the focus ring on keyboard focus; a click opens it in the default app, and only web and mail links open.
  A table has 1px `line` borders, a `panel` header row and scrolls sideways when wide.
  Inline code sits on a `raised` chip with the Inset radius.
- **Code block:** a `panel` card with a 1px `line` border and the Control radius.
  No language label.
  The copy icon button sits 4px from the top and right edges and shows only on hover or keyboard focus; it shows a check for 1.5s after it copied.
  The code scrolls sideways and never wraps.
- **Find:** ⌘F opens a bar floating at the transcript's top right (`--shadow-popover`, on `canvas` with a 1px `line-strong` border and the Control radius): a field, the position ("2 of 7", "No results"; meta, tabular), previous and next icon buttons (disabled without matches) and a close icon button.
  Return steps to the next match, Shift-Return to the previous, Esc or a click anywhere outside the bar closes it.
  Matches take an `editor-match` background; the current one adds a 2px `accent-ink` underline and scrolls to the middle.
- **Step group:** thoughts and tool calls between two messages, one row in meta text: a toggle with a chevron and one line that truncates with an ellipsis while the chevron, counts and time keep their place (Inset radius, `raised` and `ink-secondary` on hover, `selected` when pressed), folded.
  While the group is the run's newest item, the line holds the `status-running` orbit and what its newest step does: "Thinking", "Reading mod.rs", "Editing mod.rs", "Writing mod.rs", "Running cargo check", or another call's title.
  A shell command reads "Running" and the command, even one that only reads, searches or lists.
  Each new line fades in and unblurs from 2px over 400ms.
  Once the group is past, the line names its two weightiest kinds and counts the rest ("Edited 4 files, ran 1 command, 3 more"): distinct files edited or written, commands run, other tool calls, searches, files read, folders listed, thoughts, in that order, each only when there is one.
  Its tooltip holds every kind ("1 thought, read 1 file, edited 4 files, ran 1 command").
  After it, the lines the group's finished edits and writes added ("+43", `success`) and removed ("−5", `danger`), tabular, each only when not zero; a write counts its whole content as added.
  The group's total run time sits at the toggle line's right end once it reaches 5 s and the group is past.
  The toggle opens one 26px row per step, its icon 18px in, the steps' indent.
  A call's row holds its 12px kind icon in `ink-muted` (Edit and Write a pencil, Read an eye, Search a magnifier, List a folder, Run a play, any other a file), its title at 500 in `ink-secondary`, then a path from the worktree's root, or a command in the code font at 11.5px, on one line that truncates; the full detail is the row's tooltip.
  At its end sit the `status-running` orbit while the call runs, its added and removed lines once it finished, and its run time.
  A call's added and removed lines show only when two or more of the group's calls changed lines; with one, the toggle line already holds them.
  A thought's row has no icon and shows its text on one line that truncates.
  The rows only read: no hover, no press, no focus.
  A failed call's icon and title take `danger`.
  A call that ran 5 s or more, or failed, shows its run time ("55.4 s", "320 ms"), 11px `ink-muted`, tabular.
  From a minute up a time reads in minutes ("9m 16s").
  A call the run left unfinished (stopped with Esc) has an `ink-muted` title.
  Images the group's tools returned sit once in a row below the group's last line, folded or open: the same 64px thumbnails, 8px below it, in call order, opening the same lightbox.
  No images, no row.
- **Working:** a row of meta text right above the composer while the agent runs.
  It floats over the foot of the transcript, so a run's start resizes nothing; the transcript's bottom fade moves up to end at the row's top, and the text under the row hides; while it shows, the transcript ends that much lower.
  The row holds the `status-running` orbit, a verb in `ink-secondary` ending with "…" ("Cooking…", "Locking in…", "Hold my beer…", "Going feral…"), the time since the run started ("14s", "2m 5s") and "esc stops".
  The run's start time picks the verb, so it holds for the whole run.
- **Notice:** one line of `danger` meta text above the composer.
- **Compaction:** where the agent replaced the earlier messages with a summary, the transcript holds one row that reads as a step group's toggle line: the chevron and "Compacted 150K tokens" ("Compacted the context" when the agent gives no count), folded.
  The toggle opens the summary under it as Markdown at the transcript size in `ink-secondary`, indented 18px like a step.
  The messages before the summary are gone from the session, so the row is where the transcript starts.
- **Stopped:** a run the user stops with Esc ends in a left-aligned "Stopped" row of meta text in `ink-muted`, after the run's last item.
- **Sending** keeps the transcript where the reader left it; it follows the reply only when the reader sits at the end.
- **Subagent line:** when the harness has the subagents capability and reports subagent runs (pi through a subagent extension, Codex and Claude Code themselves), the lines sit in the transcript where the first run showed up, after the items that came before it, and stay there when later messages arrive or the runs end; a session's history puts them after its last item.
  They take no card: no fill, no border, the transcript's own inset, 4px between lines.
  Its lines read as the Working row does: meta text (12px, `ink-muted`, tabular), a `StatusMark` (orbit while it runs, `status-done` or `status-error` after) and, 8px after it, one joined text.
  A single run is one line, "worker · bash · 4s": name, tool and time joined with " · ", each part only when known, no leading separator.
  Two or more leaf rows (a chain, a fan-out) get a summary line first, in `ink-secondary`, "3 subagents · 12s" (the count of the leaf rows, the agents that hold no others, so a chain of three steps reads 3; then the time from the first start to now or to the last end), its mark the orbit while one runs, else `status-error` when one failed, else `status-done`; the rows follow, indented 18px per level once the summary shows (what a run holds sits one level deeper; without a summary the top rows sit flush), each "name · tool · time" with its own mark.
  A row's name is its label, or its agent when the label only repeats the step's key ("task-1").
  A parallel run named after its children's agents ("scout, scout") gets no row of its own; its children take its level.
  The time ticks while a run runs and freezes at its length once it ended.
  A chain of single children that repeat one name (a run, its step, a nested summary of the same agent) is one row at any depth: it keeps the deepest tool, the earliest start, the deepest end and one state: running while any of them runs (a nested run can outlive its parent), else failed if any failed, else done; a running row has no end, so its time keeps ticking.
  A stop or a turn's end keeps the lines, because background runs outlive the turn; the lines stay through an automatic context compaction; the extension's own empty report on `/new` or `/resume`, and the agent's exit, clear them.
  With no run reported, or once the extension clears them, nothing shows.
  Chord has no control that stops a background run.
- **Composer:** a `sheet` card with a 1px `line` border, the Card radius, 10px 10px 8px padding and `--shadow-composer`.
  The draft is 14px at 1.625.
  Focus inside changes nothing.
  The text area starts one line tall, grows with the draft up to 224px, then scrolls inside; a new pane width or text size fits it to the draft again.
  While the agent is not ready the text area is inert and its placeholder takes `ink-disabled`.
  Pasted and attached images wait above the text as 48px thumbnails with an 18px remove button on their top-right corner; a thumbnail takes the transcript thumbnail's zoom cursor and `line-strong` hover border and opens the lightbox.
  Its bar holds the attach icon button (paperclip, tooltip "Attach Images") on the left, shown only when the harness has the attachments capability.
  After it, on the left, when the harness has told its permission mode, the permissions button (tooltip "Permissions"): a plain text button with the mode's 16px icon in `ink-muted` (`ink-secondary` on hover and while its menu is open), then its name ("Workspace", "Full Access"), 6px apart, in the agent button's look and `selected` while its menu is open; while the mode never asks, the name and the icon both take `status-waiting`.
  The icons: Read Only an eye, Workspace a folder, Full Access and Bypass Permissions an open lock, Manual a shield, Accept Edits a pencil, Plan a list, Auto a shield with a check, Don't Ask a circle with a slash, any other mode the shield; it hides under 300px, and a click opens the Permissions picker as a menu above the composer (see Picker menu).
  After it and the context meter (below), when the harness has the models capability and has told its model, the agent button (tooltip "<Agent>, Model and Thinking"): the agent's 16px logo in `ink-muted`, then the current model's name in `ink-secondary` and, only when that model reasons, the thinking level capitalized in `ink-muted` (for example "Sonnet 5.5 Medium"), 6px apart: 28px tall, 8px padding on both sides, body size, no fill, `raised` on hover with the text one step brighter (`ink`, `ink-secondary`), `selected` when pressed and while its menu is open, `ink-disabled` and inert while the agent is not ready (as the attach button) and, as the permissions button, while the chat loads, one line.
  Nothing in the bar overlaps: as the bar narrows, the model name ellipsizes first, then the context meter's percentage and the permissions button hide under 300px (the icons, the ring and their tooltips stay), the thinking level under 240px and the agent button under 140px; the attach button and the send button never shrink.
  A click opens the agent menu above the composer (see Agent menu); a second click closes it.
  Before the agent button, when the harness has the context capability and has told its window, the context meter, set off from the agent button by a 1px `line-strong` divider 14px tall after it, so it never reads as part of the label: a 12px ring (2px stroke, `line-strong` track, the fill in `accent-ink`) and, 6px after it, the session's fill of the window as a percentage in 12px `ink-muted` meta text, tabular, 8px padding on both sides, with a tooltip of the tokens used and the window; from 50% the ring and the percentage take `status-waiting`, from 75% `status-error`, and the tooltip adds "nearly full".
  `/new` empties it (0%).
  A session without a message shows 0% before the harness tells its window, with the tooltip "Context: empty".
  The send button sits on the right: a 32px `accent` circle with a white up arrow, `selected` with an `ink-muted` arrow while the draft is empty.
  Its tooltip reads Send, or Steer while the agent runs and the harness has the steer capability; without it the button stays disabled until the run ends.
- **Dialog:** a dialog an agent opens (select, confirm, input) takes the composer's place as the same `sheet` card: a Heading title, then the body, and the composer returns once it is answered.
  Esc cancels it.
  A select is a list of 30px rows in `ink-secondary`, 224px tall at most, scrolling inside; the highlighted row takes `selected` and `ink`, moves with ↑ and ↓ or the pointer, and Return or a click picks it.
  A select opens with the current option highlighted, if it has one, and marks it with a 16px `ink-muted` check at the row's right end.
  Every select starts with a Field (placeholder "Filter") that narrows the rows by what they contain, and reads "No matches" in meta text when none is left.
  A select about a command or files (an approval) shows them under the title as selectable 12px code text in `ink-secondary` at 1.6, pre-wrapped, in a `panel` block with a 1px `line` inner border, the Control radius and 8px 10px padding, 160px tall at most and scrolling inside.
  An approval's rows and a confirm's rows each end with their number in a 20px `line-strong` outlined key (11px, 500, `ink-muted`, `ink-secondary` on the highlighted row), and the digit key picks that row at once.
  An option with a description shows it under its label in 12px `ink-muted`.
  A select that takes several options starts each row with a 14px checkbox, which Return or a click ticks, and ends with a primary "Submit", right-aligned, that answers with the ticked rows.
  A confirm shows its message in `ink-secondary`, or in the approval's code block when the message is a shell command, then two rows, "Yes" highlighted first and "No", without a Filter field.
  A confirm with a link shows the link under the message as selectable 12px text, and its rows read "Open" and "Decline", where Open sends the link to the default browser, whatever the link setting says, and answers.
  An input shows a Field and "Cancel" and "Submit".
- **Slash menu:** while the draft is only a slash and a command name, a card of its own floats above the composer card, 6px clear of it and as wide: `sheet`, the composer card's own background, a 1px `line` border, the Card radius, `--shadow-popover`, 6px padding, 372px tall at most (ten rows), scrolling inside with the global scrollbar (see Scrollbars) and keeping the highlighted row in view.
  Its rows are 36px with 10px padding on both sides and no dividers, one line each: a 16px `ink-muted` outline icon for the command's kind (skill, prompt, extension or session; one per kind, none per command; a session command takes the tab's chat icon), then after an 8px gap its title in `ink` at the body size (the name without a `skill:` prefix, split on `-`, `_` and `:`, each word capitalized), then a one-line `ink-muted` meta description that ends in an ellipsis, then a right-aligned `ink-muted` meta label that never shrinks and sits 24px (the 8px gap plus 16px) after the description, even when it is ellipsized: "Local file" for a skill or prompt from a file, the package name for one from a package, "Extension" for an extension command, "Session" for a session command (Model, Thinking, New, Resume, Fork, Clone, Tree, Export), which runs when sent instead of going to the agent.
  The highlighted row takes `selected` with the Control radius and its description and label step up to `ink-secondary`.
  No chevrons.
  A command whose name equals the typed part comes first, then names that start with it; the typed part of the title, when it matches from the start of a word ignoring case and separators, takes 600, and nothing is marked when it does not.
  ↑ and ↓ or the pointer move the highlight; Tab or a click completes the name and a space; Return completes it too, unless the draft already is that name, then it sends.
  Esc or a click anywhere outside the list closes it until the draft changes.
  A harness that lists no commands shows no menu.
  A message that began as `/skill:name` shows that text in its bubble, never the skill's body.
- **Agent menu:** the agent button opens a card that floats 6px above the composer card, flush with its right edge: the slash menu's card (`sheet`, 1px `line` border, Card radius, `--shadow-popover`), `min(460px, 100%)` wide, 420px tall at most, no padding of its own.
  It grows out of the button: it fades in, rises 4px and scales from 96% over 200ms with `--ease-out`, its origin the button's center on the card's bottom edge; under Reduce Motion it only fades.
  It closes at once.
  On top, when more than one agent is installed and signed in (Settings' check; the pane's own always counts), a sliding Segmented control across the card, 8px in on every side, above a 1px `line` rule that keeps scrolled rows clear of it, one segment per agent with its 16px logo and name 6px apart, the pane's agent first checked (tooltip "Runs in this pane"); a click shows that agent's models at once.
  Tab and Shift-Tab step the agents.
  Then the agent's models, as the Picker menu's rows (36px, 10px padding, `ink`, the current one marked with the dialog's check, a pi model's provider as the meta line), 252px tall at most and scrolling; Chord reads every agent's models in the background at launch and again whenever an agent runs, so another agent shows the list it last gave; an agent whose list never came back (missing, signed out) shows one row, "Default Model".
  ↑ and ↓, the pointer and typing move the `selected` highlight as in the model picker.
  Under the rows, behind a 1px `line` rule, the thinking bar: "Thinking" in 12px `ink-muted` on the left, 16px in, and at the right a sliding Segmented control of every level the agent's models have, in their order, capitalized, 12px; the levels the highlighted model lacks show disabled in `ink-disabled`, so no segment moves as the highlight moves.
  Its thumb slides to that model's level as the highlight moves over the rows (another agent's levels jump instead), hides for a model without levels, and returns to the kept highlight when the pointer leaves the list.
  An agent whose models have no levels reads "No thinking levels" there.
  The mark is the level set for that model in the bar, else the running level when the model has it, else the model's default; none when the harness names no default.
  A level click, or ← and →, sets it: on the pane's agent at once, with its model, and the card stays open.
  A row click or Return picks the model with its marked level and closes the card.
  A pick of another agent's model opens a new chat tab in the focused group, which starts that agent at the picked model and level; the pane's chat and its agent stay as they are, running or not.
  Esc or a click outside closes the card.
  `/model` and `/thinking` still open the Picker menu.
- **Picker menu:** the permissions picker, and the model and thinking level pickers that `/model` and `/thinking` open, float above the composer card, which stays in place, as the slash menu does: the same card, position, size and scrolling.
  Neither picker has a Field; its list takes the focus.
  In the model picker, typing moves the highlight as a Mac menu does: to the first row with a word that starts with the typed letters, else the first that contains them.
  When the typed letters match nothing, the longest matching tail of them wins, so typing "fable" right after "opus" lands on Fable without deleting anything.
  The rows never narrow.
  A 1s pause starts a new name, and the highlight stays put when nothing matches.
  Then come the options as 36px rows in `ink` with 10px padding, the current one marked with the dialog's check at the row's end.
  In the permissions picker each row leads with its mode's icon in `ink-muted`, `ink-secondary` on the highlighted row, 8px before the name.
  ↑ and ↓ or the pointer move the `selected` highlight, Return or a click picks.
  A select whose rows take a More action, as the branch menu's do, ends each row with a 20px More icon button (three dots, Inset radius, tooltip "More"), shown while the row is highlighted, under the pointer or holds the focus, fading in over 120ms with `--ease-out`; `raised` on hover, `control-selected` on the highlighted row (see `design/changes-view.md`).
  Esc or a click anywhere outside the card closes it without a pick.
  Replies and tool calls from the agent leave it open; only the agent's exit closes it.
- **Jump to latest:** once the reader scrolls 72px or more above the end of the transcript, a 28px `canvas` circle with a 1px `line-strong` border, the control shadow and an `ink-secondary` down arrow sits centered 4px above the Working row's place, whether the row shows or not.
  It shows no status: the Working row does.
  Hover takes `raised` and `ink`, pressed takes `selected`.
  A click scrolls smoothly to the end and resumes following the reply.
  It fades in and rises 8px from 90% scale over 200ms with `--ease-out`, fades out the same way once the reader is back at the end, and presses to 94% scale.
- **Empty state:** a centered title, one muted line and the primary button.
- **Harness that cannot start:** while no agent runs and the pane's harness is missing or older than Chord needs, its harness row (`design/onboarding.md`) takes the Start button's place, 440px at most in the empty state; a pane without a harness shows every row while no harness can be the default.
  The empty state's line reads "Run the command, then switch back. Chord checks again." Under "The agent stopped." the row sits full width, 8px below the line.
  The rows hide the notice, which says the same.
  The check runs when the pane's agent goes idle, and while the rows show, each time the window comes forward.
- **Starting:** while an agent starts in an empty pane, a 14px `ink-muted` spinner sits centered in the transcript ("Starting the agent" for VoiceOver).
  A new chat whose harness opened a session before in this run shows no spinner: its composer bar shows what that session opened with (model, thinking level, permission mode, context meter, attach button) until the agent answers.
