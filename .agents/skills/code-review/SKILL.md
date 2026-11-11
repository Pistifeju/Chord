---
name: code-review
description: "Two-axis review of a change: Standards (does the code follow this repo's documented rules?) and Spec (does it do what the issue or spec asked?). Works on uncommitted changes, staged changes, a commit range, a branch, or a PR. Use when the user asks to review current changes, work in progress, a branch, a PR, or \"since X\"."
---

Two-axis review of one diff:

- **Standards**: does the code follow this repo's documented coding rules?
- **Spec**: does the code do what the originating issue or spec asked for?

Each axis runs in its own sub-agent with a fresh context, so neither sees the other's reasoning. This skill stands alone: it uses git, the file system, and the sub-agent tool. It calls no other skill.

## 1. Pin the scope

Map the user's words to one scope and its diff command:

| User says | Scope | Diff command |
| --- | --- | --- |
| "current changes", "uncommitted", "WIP", or nothing while the working tree is dirty | working tree vs `HEAD` | `git diff HEAD` |
| "staged" | index vs `HEAD` | `git diff --cached` |
| a ref (`main`, a SHA, a tag, `HEAD~5`) | everything since the merge-base, committed and uncommitted | `git diff $(git merge-base <ref> HEAD)` |
| "only commits since <ref>" | committed work only | `git diff <ref>...HEAD` |
| a PR number | the PR | `gh pr diff <n>` |
| nothing, and the working tree is clean | ask the user for a ref | |

For working-tree scopes, also list untracked files with `git ls-files --others --exclude-standard`. They count as new files in the diff, and the sub-agents read them in full.

For ref scopes, record the commits with `git log <ref>..HEAD --oneline`.

Done when: the ref resolves (`git rev-parse <ref>`), and the diff plus untracked files is non-empty. Stop and tell the user if either fails.

## 2. Find the spec

Take the first source that exists:

1. A path, issue, or description the user gave in this conversation.
2. Issue references in the commit messages or branch name (`#123`, `Closes #45`). Fetch them with `gh issue view <n>` when `gh` is installed.
3. A spec file in the repo matching the feature or branch name: `SPEC.md`, or files under `docs/`, `specs/`, `.scratch/`.
4. Ask the user. If they say no spec exists, skip the Spec axis.

## 3. Find the standards

List every file in the repo that documents how to write code: `AGENTS.md`, `CLAUDE.md`, `CODING_STANDARDS.md`, `CONTRIBUTING.md`, design docs the agent files point to. Include them whenever they exist.

The Standards axis also carries the **smell baseline** below, Fowler's code smells from _Refactoring_, ch. 3. Two rules bind it:

- **The repo overrides.** Where a documented repo rule endorses something a smell would flag, suppress the smell.
- **Always a judgement call.** Label each smell a heuristic ("possible Feature Envy"). Skip anything a linter or formatter already enforces.

Each smell reads *what it is* → *how to fix*:

- **Mysterious Name**: a name that hides what the thing does or holds. → rename it; if no honest name comes, the design is murky.
- **Duplicated Code**: the same logic shape in more than one hunk or file. → extract the shared shape, call it from both.
- **Feature Envy**: a method that reaches into another object's data more than its own. → move the method onto that data.
- **Data Clumps**: the same few fields or params travelling together. → bundle them into one type.
- **Primitive Obsession**: a primitive standing in for a domain concept. → give the concept its own small type.
- **Repeated Switches**: the same `switch`/`if` cascade on the same type at several sites. → polymorphism, or one shared map.
- **Shotgun Surgery**: one logical change forces scattered edits across many files. → gather what changes together into one module.
- **Divergent Change**: one module edited for several unrelated reasons. → split it so each module changes for one reason.
- **Speculative Generality**: abstraction, params, or hooks for needs the spec lacks. → delete and inline until a real need shows.
- **Message Chains**: long `a.b().c().d()` navigation. → hide the walk behind one method on the first object.
- **Middle Man**: a class or function that mostly delegates. → call the real target direct.
- **Refused Bequest**: a subclass that ignores most of what it inherits. → swap inheritance for composition.

## 4. Run both reviews in parallel

Start both sub-agents in one call, in the foreground, each with a fresh context. Every brief opens with this guardrail, verbatim:

> Perform this review yourself, using only git commands and file reads. Do not invoke any skill and do not start sub-agents.

**Standards brief** contains:

- the diff command, the untracked file list, and the commit list if any;
- the standards files from step 3, and the smell baseline pasted in full;
- the task: "Report, per file or hunk, (a) each place the diff breaks a documented rule: cite the file and the rule; (b) each baseline smell you spot: name it and quote the hunk. Mark documented-rule breaches as hard or judgement call; smells are always judgement calls. Skip what tooling enforces. Under 400 words."

**Spec brief** contains:

- the diff command, the untracked file list, and the commit list if any;
- the spec path or its fetched text;
- the task: "Report (a) requirements the spec asks for that are missing or partial; (b) behaviour in the diff the spec did not ask for; (c) requirements implemented wrong. Quote the spec line for each finding. Under 400 words."

If no sub-agent tool exists, run the two reviews yourself one after the other, Standards first, and finish one report before you start the other.

## 5. Report

Print the two reports under `## Standards` and `## Spec`, verbatim or lightly cleaned. Keep each axis's findings in its own block and order.

Close with one line: findings per axis, and the worst issue within each axis. Name no winner across axes, since a change can pass one axis and fail the other.
