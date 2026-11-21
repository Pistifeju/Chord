# The updater

Chord learns of a newer release from GitHub, and a click downloads it and restarts into it.

## 1. Decisions

1. **Tauri's updater plugin, called from Rust.** `check_update` and `install_update` in `commands/app.rs` are the whole surface. The window gets a version string and the download's percentage and calls no plugin API, so the capability file stays as it is.
2. **The feed is one file on the latest release.** `release.yml` writes `latest.json` with the version, the archive's URL and its signature, and attaches it beside `Chord.app.tar.gz`. The app reads `releases/latest/download/latest.json`. No server.
3. **The user starts the restart.** A restart stops every agent, shell and build, so Chord never installs on its own. The status bar shows `Update to 0.2.1`; a click installs, after the confirm sheet while an agent works, the same rule Quit follows. The button holds the progress: `Downloading 42%` from a channel `install_update` writes to, then `Restarting`.
4. **Look at launch and every six hours.** Chord stays open for days, and a launch-only look would miss releases.
5. **A failed look is silent.** Offline, or a latest release that has no feed, leaves the button as it was. A failed download shows a failure toast and stops nothing.
6. **A dev build never looks.** It has no app bundle to replace.
7. **Only the release workflow builds the update archive.** `createUpdaterArtifacts` comes from a `--config` argument there, so `npm run app:install` needs no update key.
8. **The workflow checks the signing key.** Tauri only warns when the private key in the secrets is not the pair of the public key in `tauri.conf.json`. The workflow compares the two key ids and stops before it publishes.

## 2. Out of scope

- Release notes in the app.
- A "Check for Updates" menu item.
- A toast, a sidebar row or a floating card for the notice. The status strip's text item won the comparison: a card in the bottom right corner covers the composer's send button.
- Intel Macs. The feed holds `darwin-aarch64` alone, like the disk image.

## 3. The keys

The update key pair is separate from the Apple certificate. The public key sits in `tauri.conf.json` under `plugins.updater.pubkey`. The private key and its password are the repo secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. An app that trusts a lost key can never update itself again: its users download the next version by hand.

## 4. Proof

The first release with the updater proves nothing alone. Install it, publish the next version, and the installed app must show the button and restart into that version.
