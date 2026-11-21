# Contributing

## Run

- `npm install`, once.
- `npm run tauri dev` starts the app with live reload.
- `npm run check` runs every check: types, tests, formatting, lints.

## Change

- One feature per pull request.
- `AGENTS.md` holds the rules of the codebase, for people and for coding agents. `ARCHITECTURE.md` says where code goes and `DESIGN.md` how the UI looks.
- `npm run check` must pass.

## Agent skills

- `.agents/skills/` holds the skills a coding agent can run here. `code-review` works in any clone.
- `polish-ui`, `variants` and `variants-ask` read the Impeccable skill, which git ignores. Run `npx impeccable install` in the project folder and pick the project install, so it lands in `.agents/skills/impeccable/`. Source: https://impeccable.style/
