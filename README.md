<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="" width="96" />
</p>

<h1 align="center">Chord</h1>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-1b4ed8?style=flat" alt="License: MIT" />
  <img src="https://img.shields.io/badge/macOS-14%2B-1b4ed8?style=flat" alt="macOS 14 or later" />
  <img src="https://img.shields.io/badge/status-early%20development-bb4d00?style=flat" alt="Status: early development" />
</p>

<h2 align="center">Parallel coding agents for iOS and macOS, each with its own worktree.</h2>

<p align="center">
  Built for iOS and macOS developers who ship with coding agents. Every branch has its own DerivedData and its own Simulator, so you can test one build while an agent installs another. Bring your own harness and run your app next to the chat. No more juggling Xcode, Terminal and a git client.
</p>

<h3 align="center"><a href="https://github.com/Pistifeju/Chord/releases/latest"><ins>Download Chord</ins></a> &nbsp;·&nbsp; <a href="CONTRIBUTING.md"><ins>Contribute</ins></a></h3>

![Chord with a project's worktrees in the sidebar, two agent chats side by side and the Simulator on the right](docs/assets/readme/screenshot.png)

## Your app, right next to your agents

The Simulator lives inside Chord, beside the agent writing the code. Three agents can build the same Xcode project at once and nobody's build breaks.

Every `#Preview` sits one click away. When an agent changes a SwiftUI file, the preview updates on its own.

## Features

- **Live Simulator.** Your app runs inside Chord, one simulator per branch.
- **SwiftUI previews.** Every `#Preview` in one list, updated as the agent edits.
- **My Mac.** Build and run your macOS app from Chord.
- **A worktree per branch.** Run agents in parallel without one overwriting another's files.
- **Status at a glance.** Running, waiting for you, done or failed, for every chat.
- **Split panes.** Several agents, side by side, on one branch.
- **Changes view.** Review the diff, stage, stash and switch branches.
- **Terminal and editor.** A shell and an editor for quick fixes.
- **Usage.** See how many tokens every pi, Codex and Claude Code session burned.

## Supported harnesses

<p>
  <a href="https://pi.dev"><kbd><img src="https://pi.dev/favicon.svg" alt="" width="16" valign="middle" /> pi</kbd></a> &nbsp;
  <a href="https://github.com/openai/codex"><kbd><img src="https://www.google.com/s2/favicons?domain=openai.com&sz=64" alt="" width="16" valign="middle" /> Codex</kbd></a> &nbsp;
  <a href="https://docs.anthropic.com/claude/docs/claude-code"><kbd><img src="https://www.google.com/s2/favicons?domain=claude.ai&sz=64" alt="" width="16" valign="middle" /> Claude Code</kbd></a> &nbsp;
  <a href="https://opencode.ai"><kbd><img src="https://www.google.com/s2/favicons?domain=opencode.ai&sz=64" alt="" width="16" valign="middle" /> OpenCode</kbd></a>
</p>

## Requirements

- macOS 14 or later
- Xcode
- At least one of [pi](https://github.com/earendil-works/pi), [Codex](https://github.com/openai/codex), [Claude Code](https://docs.anthropic.com/claude/docs/claude-code) or [OpenCode](https://opencode.ai)

## License

MIT
