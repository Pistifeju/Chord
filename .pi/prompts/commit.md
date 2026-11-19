---
description: Commit the current changes, split into one commit per feature
argument-hint: "[what to commit]"
---
Commit ${@:-every current change}.

1. Run `git status --short && git diff --stat && git log --oneline -5`. Read `git diff` only for files whose change you do not know from this session.
2. Leave out changes another session made that do not belong to this task. Name them in your reply.
3. Split by feature folder: one commit per area. Stage with `git add <paths>`, never `git add -A` when the tree holds unrelated work.
4. Message: `type(area): what changed`, lower case. Types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `chore`. You may leave out the area.
5. Commit with `git commit -qm`. Do not push. Do not run `npm run check`.
6. Reply with `git log --oneline -<n>` for the new commits and `git status --short`.
