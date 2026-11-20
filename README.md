# Chord

A macOS workspace for coding agents. Each task gets its own git worktree, and each worktree gets its own agent chats. Chord works with any git repo; a worktree with an Xcode project also gets build, run, simulator and SwiftUI preview tools.

Chord is in early development.

## Requirements

- macOS 14 or later
- Xcode Command Line Tools (`xcode-select --install`)
- [Rust](https://rustup.rs); rustup installs the version `rust-toolchain.toml` pins
- [Node.js](https://nodejs.org) 24 or later, with npm
- A coding agent: [pi](https://github.com/earendil-works/pi), Codex or Claude Code. Chord shows the install command of a missing one.

## Build

```sh
git clone https://github.com/Pistifeju/Chord.git
cd Chord && npm install
npm run tauri build
```

The app lands in `target/release/bundle/macos/Chord.app`.

## Develop

`npm run tauri dev` starts the app with live reload. `CONTRIBUTING.md` has the rest.

## License

MIT
