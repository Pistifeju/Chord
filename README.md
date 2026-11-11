# Chord

A macOS workspace for coding agents. Each task gets its own git worktree, and each worktree gets its own agent chats. Chord works with any git repo; a worktree with an Xcode project also gets build, run, simulator and SwiftUI preview tools.

Chord is in early development. `SPEC.md` lists the features and marks the ones that work.

## Requirements

- macOS
- Xcode Command Line Tools (`xcode-select --install`)
- [Rust](https://rustup.rs)
- [Node.js](https://nodejs.org) with npm
- [pi](https://github.com/earendil-works/pi), the coding agent Chord runs

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
