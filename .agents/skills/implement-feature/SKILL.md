---
name: implement-feature
description: "Implement one ticket end to end with a fixed chain of subagents: implement, polish-ui, improve-animations, code-review. Use when the user names a ticket and where to implement it from."
disable-model-invocation: true
---

You are the main agent. You write no code yourself. You run four subagents one after another and report to the user. The user names the ticket and its source (a file, a `SPEC.md` row, an issue text, a link).

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

### Subagent setup

| Role | Tier | pi (pi-subagents) | Claude Code | Codex |
|------|------|-------------------|-------------|-------|
| Writer (A to C) | Sonnet, medium effort | agent `worker`, model `anthropic/claude-sonnet-5-5:medium`, `async: false` | Task tool, `general-purpose`, model `sonnet` | spawn an agent, model `gpt-6-luna`, reasoning effort `medium` |
| Reviewer (D) | Opus, medium effort | agent `reviewer`, model `anthropic/claude-opus-5-5:medium`, `async: false` | Task tool, `general-purpose`, model `opus` | spawn an agent, model `gpt-6.1-sol`, reasoning effort `medium` |

Another harness: pick its closest equivalent of the tier. The reviewer must be able to start its own subagents, because the `code-review` skill does.

### A. Implement

- Writer.
- Task: implement the ticket. Tick the feature in `SPEC.md`. Return the list of changed files and the `npm run check` result.

### B. Polish UI

- Writer.
- Task: read `.agents/skills/polish-ui/SKILL.md` and run it on the current changes (no target named: the skill takes them from `git diff HEAD` and untracked files). Return the findings it did not fix and the `npm run check` result.
- Skip this step when step A touched no `.tsx` or `.css` file. Say so in the report.

### C. Improve animations

- Writer.
- Task: read `.agents/skills/improve-animations/SKILL.md` and `AUDIT.md` and run the skill, limited to the motion code in the files of the current changes (`git diff HEAD --name-only` plus untracked files). Name that file list in the task so the skill does not answer with its greeting. Return the findings it did not fix and the `npm run check` result.
- Skip this step when the current changes contain no motion code. Say so in the report.

### D. Code review

- Reviewer.
- Task: read `.agents/skills/code-review/SKILL.md` and run it on the uncommitted changes, with the ticket text and source path as the spec. The reviewer edits nothing. Return the full findings on both axes (Standards, Spec), each with file and line.

## 3. Report to the user

Answer in this order:

1. Verdict: did the reviewer find anything? Yes or no, with the count per axis.
2. What now works: the ticket's behavior in concrete terms, and the changed files.
3. Each step: done, skipped (why) or failed (why), and the `npm run check` result of every editing step.
4. The reviewer's findings, ranked, each with file and line.
5. Findings from polish and animations that nobody fixed.

Do not fix reviewer findings. Do not commit. Offer one next action: fix the top finding, or commit.
