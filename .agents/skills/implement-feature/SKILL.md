---
name: implement-feature
description: "Implement one ticket end to end with a fixed chain of subagents: implement with polish-ui and improve-animations, code-review, then one fix round. Use when the user names a ticket and where to implement it from."
disable-model-invocation: true
---

You are the main agent. You write no code yourself. You run three subagent steps one after another and report to the user. The user names the ticket and its source (a file, a `SPEC.md` row, an issue text, a link).

## 1. Pin the ticket

- The user named a ticket and a source: read the source. That is the ticket.
- Either is missing: ask the user for it and stop.

Done when: you hold the ticket text and the source path.

## 2. Run the chain

Rules for every subagent:

- Launch each one with your harness's subagent mechanism, using the role and model tier from the table below. Wait for it to return before you start the next.
- Your harness has no subagent mechanism, or cannot set the model tier: stop and tell the user. Do not run the steps yourself.
- Fresh context. Put everything it needs in the task: the ticket text, the source path, the steps it runs, the rules below.
- Task text for every subagent includes: follow `AGENTS.md`; no commits, no pushes, no screenshots, no launching the app; run `npm run check` at the end of an editing step and report the result.
- A subagent fails or `npm run check` stays red: stop the chain, report the exact failure to the user.
- Gate the Writer's report before step B. It passes when every skipped part names its allowed skip condition, and every run part returns the evidence its bullet asks for, covering every file in its target. A part misses the gate: launch the Writer once more for that part only, and name the gap in the task. It misses again: stop the chain and report the gap.

### Subagent setup

| Role | Tier | pi (pi-subagents) | Claude Code | Codex |
|------|------|-------------------|-------------|-------|
| Writer (A) | Sonnet, medium effort | agent `worker`, model `anthropic/claude-sonnet-5-5:medium`, `async: false` | Task tool, `general-purpose`, model `sonnet` | spawn an agent, model `gpt-6-luna`, reasoning effort `medium` |
| Reviewer (B) | Opus, medium effort | agent `worker`, model `anthropic/claude-opus-5-5:medium`, `async: false` | Task tool, `general-purpose`, model `opus` | spawn an agent, model `gpt-6.1-sol`, reasoning effort `medium` |
| Fixer (C) | same as Writer (A) | same as Writer (A) | same as Writer (A) | same as Writer (A) |

Another harness: pick its closest equivalent of the tier. The reviewer needs shell and edit tools, because it fixes what it finds. In pi the `reviewer` agent is read-only, so step B uses `worker`. The `code-review` skill asks for two sub-agents, one per axis: where the reviewer cannot start subagents, it runs both axes itself, one after the other.

### A. Implement, polish, animate

- One Writer runs all three parts in one context, in this order, so it reuses the files it already read.
- Part 1, implement: implement the ticket. Tick the feature in `SPEC.md`.
- Part 2, polish: the one skip condition is that part 1 touched no `.tsx` or `.css` file under `src/`; name it when you skip. Otherwise read `.agents/skills/polish-ui/SKILL.md` in full and run every step on the target its step 1 pins (no target named: the skill takes it from `git diff HEAD` and untracked files). Evidence: the target file list, then for layout, typeset, polish and the drift sweep, each finding with file and selector, or `none` with the files checked.
- Part 3, animations: the one skip condition is a grep of the changed files for `transition`, `animation`, `@keyframes`, `transform` and `--ease` that returns no hit; name the grep and its empty result when you skip. Otherwise read `.agents/skills/improve-animations/SKILL.md` and `AUDIT.md` in full and run the skill, limited to the motion code in the changed files (`git diff HEAD --name-only` plus untracked files). Name that file list to the skill so it does not answer with its greeting. Evidence: the file list, then each finding with file and line, or `none` with the files checked.
- Return the list of changed files, the evidence of parts 2 and 3, the findings it did not fix, and one `npm run check` result at the end.

### B. Code review

- Before you launch it, run `git diff HEAD --stat` and `git status --short` yourself, and put both outputs in the task, with the Writer's list of changed files. The reviewer starts from them instead of searching for the changes.
- Reviewer.
- Task: read `.agents/skills/code-review/SKILL.md` and run it on the uncommitted changes, with the ticket text and source path as the spec. Then fix only the trivial findings: a one-line, mechanical fix with no behavior change (an unused import, a stray comment, a naming rule, a leftover debug line). Leave every other finding untouched, and list it. Run `npm run check` after the fixes. Return every finding on both axes (Standards, Spec), each with file and line and the status `fixed` or `open`, plus the `npm run check` result.

### C. Fix the open findings

- No `open` finding: skip C and name the skip.
- Fixer, one launch. Task: the ticket text and source path, then every `open` finding from B verbatim, with file, line and axis. Fix each one. A finding that needs a product or scope decision (a new behavior the ticket does not name, a choice between two designs): leave it open and name the decision. Run `npm run check` at the end. Return each finding with the status `fixed` or `needs decision` (with the decision), the changed files, and the `npm run check` result.
- Done when: every `open` finding from B carries a C status.

## 3. Report to the user

Answer in this order:

1. Verdict: did the reviewer find anything? Yes or no, with the count per axis, and how many it fixed.
2. What now works: the ticket's behavior in concrete terms, and the changed files.
3. Each step: done, skipped (why) or failed (why), and the `npm run check` result of every editing step.
4. The reviewer's findings, ranked, each with file, line and final status: `fixed` (by B or C) or `needs decision` (with the decision). Fixes from C had no review: say so.
5. Findings from polish and animations that nobody fixed.

Do not fix findings yourself. Do not commit. Offer one next action: decide the top `needs decision` finding, or commit.
