<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="" width="96" />
</p>

<h1 align="center">Chord</h1>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-1b4ed8?style=flat" alt="License: MIT" />
  <img src="https://img.shields.io/badge/macOS-14%2B-1b4ed8?style=flat" alt="macOS 14 or later" />
  <img src="https://img.shields.io/badge/status-early%20development-bb4d00?style=flat" alt="Status: early development" />
</p>

<h2 align="center">Coding agents and the iOS Simulator in one window.</h2>

<p align="center">
  Built for iOS developers who ship with coding agents. Every task gets its own git worktree and its own Simulator, so your agents don't step on each other's code or builds. Put pi, Codex or Claude Code on it and watch your app come to life right next to the chat. No more juggling Xcode, Terminal and a git client.
</p>

<h3 align="center"><a href="https://github.com/Pistifeju/Chord/releases/latest"><ins>Download Chord</ins></a> &nbsp;·&nbsp; <a href="CONTRIBUTING.md"><ins>Contribute</ins></a></h3>

![Chord with a project's worktrees in the sidebar, two agent chats side by side and the Simulator on the right](docs/assets/readme/screenshot.png)

## Your app, right next to your agents

The Simulator lives inside Chord, beside the agent writing the code. Each worktree gets its own simulator and its own DerivedData, so three tasks can build the same Xcode project at once and nobody's build breaks.

Every `#Preview` in the worktree sits one click away. When an agent changes a SwiftUI file, the preview updates on its own.

## Features

- **Live Simulator.** Your app runs inside Chord, one simulator per worktree.
- **SwiftUI previews.** Every `#Preview` in one list, updated as the agent edits.
- **A worktree per task.** Run tasks in parallel without agents overwriting each other.
- **Status at a glance.** Running, waiting for you, done or failed, for every chat.
- **Split panes.** Several agents, side by side, in one worktree.
- **Changes view.** Review the diff, stage, stash and switch branches.
- **Terminal and editor.** A shell per worktree and an editor for quick fixes.
- **Usage.** See how many tokens every pi, Codex and Claude Code session burned.
- **Any git repo.** Not building for iOS? You still get everything else.

## Supported harnesses

<p>
  <a href="https://pi.dev"><kbd><img src="https://pi.dev/favicon.svg" alt="" width="16" valign="middle" /> pi</kbd></a> &nbsp;
  <a href="https://github.com/openai/codex"><kbd><img src="https://www.google.com/s2/favicons?domain=openai.com&sz=64" alt="" width="16" valign="middle" /> Codex</kbd></a> &nbsp;
  <a href="https://docs.anthropic.com/claude/docs/claude-code"><kbd><img src="https://www.google.com/s2/favicons?domain=claude.ai&sz=64" alt="" width="16" valign="middle" /> Claude Code</kbd></a>
</p>

## Requirements

- macOS 14 or later
- Xcode
- At least one of [pi](https://github.com/earendil-works/pi), [Codex](https://github.com/openai/codex) or [Claude Code](https://docs.anthropic.com/claude/docs/claude-code)

## License

MIT
