# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

A solo developer, often an iOS developer, who runs several pi agents in parallel on separate tasks and switches between them. They open Chord to start a task, check which agent needs them, answer it, and move to the next.

## Product Purpose

Chord is a macOS workspace for coding agents. Each task gets its own git worktree, and each worktree gets its own pi agent chats. Success: the developer runs many agents at once and always knows which one is waiting, done or broken, without losing track of any task.

## Positioning

- One worktree per task, with an agent status Rust decides and the sidebar shows per worktree and pane.
- Built-in iOS tools: build, run, simulator and SwiftUI previews, shown only in a worktree with an Xcode project.

## Operating Context

- Works with any git repo. The developer adds a project, creates a worktree per task, and splits panes so each pane runs its own agent.
- pi is the agent harness Chord runs. Xcode Command Line Tools, git and pi must be installed; Chord tells the user how to install a missing one.
- The developer stays in one window for hours, switching worktrees with ⌘1-9 and keyboard shortcuts.

## Capabilities and Constraints

- Native macOS app (macOS 14+), built with Tauri 2, Rust and React/TS. The UI renders in a webview and follows macOS conventions: transparent title bar, traffic lights, the bundled Inter font, light and dark appearance.
- `SPEC.md` lists every feature and ticks the ones that work. Section 15 lists what Chord leaves out (pull requests, command palette, profiles, and more).
- Features build only what they need: no empty containers, no options nobody asked for.
- iOS UI shows only in a worktree with an Xcode project.

## Brand Commitments

- Name: Chord. MIT licensed, open source, early development.
- Visual direction: the category standard for agent workspaces. The developer chose it over an invented visual world.

## Evidence on Hand

- No users, testimonials, benchmarks or press yet. Future work must not invent them.

## Product Principles

1. The developer always knows which agent needs them next.
2. A task maps to a worktree; nothing from one task leaks into another.
3. iOS tools appear where an Xcode project exists and stay out of the way elsewhere.
4. Chord keeps working when a tool or extension is missing; it says what to install.
