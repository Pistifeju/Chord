---
name: variants-ask
description: Ask one round of questions about a UI request, then build four versions of it with the variants skill.
disable-model-invocation: true
---

The `variants` skill with an interview in front of it.

## 1. Read before you ask

Read `.agents/skills/variants/SKILL.md` in full and run its steps 1 and 2. Each open point those steps tell you to decide yourself: hold it as a question candidate instead.

Done when: you hold the thing, the spot, the identity lock, the kit, and the list of question candidates.

## 2. Interview

Read Phase 1 of `.agents/skills/impeccable/reference/shape.md` and follow its Cadence.

- Ask the two or three questions whose answers change the four versions most. Draw them from your candidates and from the two rounds in Phase 1.
- Ask what the code, `DESIGN.md` and `PRODUCT.md` left open. For each question, state your likeliest reading so the user can confirm it in a word.
- Wait for the answers. A second round runs only when an answer opens a gap that changes the plan.

Done when: every question has an answer, or the user told you to go on.

## 3. Build

Run `variants` steps 3 to 8. Every answer is a constraint all four versions honor. The interview is over: from here the `variants` rule holds, and you ask nothing more.
