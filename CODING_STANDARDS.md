# Coding standards: the judgement calls a reviewer checks in a diff, on top of `AGENTS.md`

## Docs

- A change to a harness adapter updates `docs/<harness>-adapter.md` in the same diff: its commands, the records it reads, and what it now handles or still leaves unchecked.
- A doc names the file that holds a list (commands, records, capabilities) and leaves the entries to the code. A doc that copies the entries is stale once the code adds one.
- A plan's build-order notes ("after stage 2 and before stage 3") go once the last stage lands. The doc keeps the decisions and their reasons.
