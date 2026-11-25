# Mac preview

A `#Preview` of a macOS view shows in the Preview view, in the place the simulator's screen takes for iOS, drawn as a window with its title bar.
Sections 2 and 4 hold the first spike and its results, section 6 the input spike and its results, section 7 the build plan.

## 1. The design under test

1. Chord starts the built Mac app as `mac.rs` does, with a preview helper in `DYLD_INSERT_LIBRARIES`.
2. The helper puts the picked preview into an `NSWindow` titled with the preview's name and orders it in on a screen with `alphaValue = 0`, so the window takes input and nobody sees it (section 6, I1).
3. The helper sends the id of the window's Core Animation context to Chord.
4. Chord shows that context through a `CALayerHost` under the webview and draws the window's shadow. The context arrives with its corners rounded (section 4, Q2).
5. A save reloads the view through `hot_reload.rs`, as on iOS.

`CAContext` and `CALayerHost` are private API. Chord already loads private API for the simulator (`simulator_kit.rs`, `hid/`).

## 2. Questions

Run them in this order. Each one is cheaper than the next, and a failed one can end the spike.

### Q1. Do the symbols exist on macOS?

Read-only, no code.

- Search the macOS SDK's `DeveloperToolsSupport.tbd` for the three `Preview.init` variants: the SwiftUI one `ChordPreview.swift` interposes today, and the AppKit ones for `NSViewController` and `NSView`.
- Build a Mac app with Enable Debug Dylib Support on and check that its stub executable names `__previews_injection_jit_link_entrypoint`.

Pass: the SwiftUI symbol and the entry point exist. A missing AppKit variant drops `NSView` previews from the first version and nothing else.
Fail: no entry point. The helper then needs another way to take over `main`, and the spike stops until someone finds it.

### Q2. Does an off-screen window show in another process?

Two throwaway Swift files in a temp folder, built with `swiftc`, outside the repo.

- `agent`: an accessory app with one titled `NSWindow`. Its content is a SwiftUI view with a counter that ticks each second, over a `.regularMaterial` background. It prints a context id and never orders the window onto a screen.
- `host`: a visible window whose content layer is a `CALayerHost` for the id it gets as an argument.

Try these ways to get a context that holds the whole window, title bar included, and stop at the first that passes:

1. The context the window's own layer tree lives in, window never ordered in.
2. The same, window ordered in at a frame outside every screen.
3. A remote `CAContext` the agent creates, with the window's frame view layer moved into it.

Pass, all five by eye on the host window:

- the title bar, the traffic lights and the content show;
- the counter ticks while no agent window is on a screen;
- the material draws;
- a resize of the agent's window from code reaches the host;
- the agent shows no window, no Dock icon and nothing in Mission Control.

Fail: no way passes. Fall back to a window the helper orders in for real and ScreenCaptureKit in Chord, which costs the user a Screen Recording grant, and decide then whether that price is acceptable.

Record for the build: which way passed, the calls it took, and whether the title bar draws active or inactive.

### Q3. Does a signed debug build load the helper?

One hand-built `.app` in the temp folder, signed ad hoc in four ways: plain, hardened runtime, App Sandbox, both. Each gets `get-task-allow`, as Xcode's Debug builds do.

For each signature check three things:

- a dylib in `DYLD_INSERT_LIBRARIES` loads from a folder outside the bundle, proven by a line its initializer prints;
- the app can `dlopen` a second ad hoc dylib from that folder, as a hot reload does;
- a `socketpair` end the parent passes at spawn carries one request and one reply.

Then read the entitlements of a real Debug build with `codesign -d --entitlements -` and confirm the hand-built case that matches it behaves the same on that app.

Pass: all three hold for the signature a real Debug build carries.
Fail on the sandbox alone: record where the dylibs must live (inside the bundle or the app's container) and keep going, since the inherited socket already avoids the temp folder.
Fail on hardened runtime: the preview needs an entitlement or a build setting in the user's project. Record which one, because section 3.2 of `docs/ios.md` forbids a setting only Chord's build uses.

## 3. Out of scope

Out of the first spike, and covered by sections 6 and 7 since:

- Clicks and keys in the preview.
- Popovers, menus, sheets and alerts, which are windows of their own.
- The macOS driver line in `compile_commands.rs`, the session changes in `preview_session.rs` and the Preview view's UI.

Out of the feature:

- Mac Catalyst and Designed for iPad.

## 4. Results

Each question gets its verdict, the Xcode and macOS versions it ran on, and the facts section 2 asks to record.
All runs: Xcode 27.1 (27A9275), macOS 27.0 (26A428), arm64.
The spike passed: the design in section 1 holds, with the changes Q2 and Q3 record.

### Q1. Pass

The symbols live in `SwiftUI.tbd` and `AppKit.tbd` of the macOS SDK, not in `DeveloperToolsSupport.tbd`, the same split iOS has with `UIKit.tbd`.

- SwiftUI: `$s21DeveloperToolsSupport7PreviewV7SwiftUIE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdAD0J0_pyScMYcctcfC`, the same name `ChordPreview.swift` interposes on iOS.
- `NSViewController`: `$s21DeveloperToolsSupport7PreviewV6AppKitE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdSo16NSViewControllerCyScMYcctcfC`.
- `NSView`: `$s21DeveloperToolsSupport7PreviewV6AppKitE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdSo6NSViewCyScMYcctcfC`.

`ENABLE_DEBUG_DYLIB = YES` produces `Demo`, `Demo.debug.dylib` and `__preview.dylib` only in a `-scheme` build; a `-target` build produces no debug dylib.
The stub names `__previews_injection_jit_link_entrypoint`.
A dylib in `DYLD_INSERT_LIBRARIES` that exports it gets called with `argc` and `argv`, with no other environment variable set.

### Q2. Pass, way 1

The context the window's own layer tree lives in passes all five checks, with the window never ordered in:

- the title bar, the traffic lights and the SwiftUI content show;
- the counter ticks;
- the material draws, in its inactive look;
- a `setContentSize` in the agent reaches the host;
- the agent shows no window in Mission Control and no Dock icon.

The calls it took:

1. `NSApp.setActivationPolicy(.accessory)`.
2. `NSWindow(contentRect:styleMask:backing:defer: false)` with `.titled`, never ordered in.
3. The context in `CAContext.allContexts()` whose `layer.name` is `NSCGSWindow`. Its root layer holds a `CABackdropLayer` and an `NSCGSWindowBackingStoreLayer`, and it exists before any order-in.
4. Its `contextId` goes to the host, which sets it on a `CALayerHost`.

The host needs no `CAHostingToken`: a `CALayerHost` with the bare `contextId` showed the same picture as one with the token.
The bare id fits in the inherited socket, so no mach port has to cross.

The title bar draws inactive.
The window's root layer sets `cornerRadius = 16` and `masksToBounds`, so the context arrives with its corners rounded; `screen_view.rs` draws the shadow only, and no corner mask.
The spike ran with `defer: false` only.

The pixel capture of the host's own window (`CGWindowListCreateImage`) showed the hosted area empty while the screen showed the content: without a Screen Recording grant, the capture drops another process's content.
A check of this design has to happen by eye.

### Q3. Pass, with one condition

Xcode drops the hardened runtime from an ad hoc (Sign to Run Locally) build: `ENABLE_HARDENED_RUNTIME = YES` in the project resolves to `NO`, on the command line too.
A Debug build therefore carries one of two signatures:

- Ad hoc: `app-sandbox`, `files.user-selected.read-only`, `get-task-allow`, no runtime flag. It matches the hand-built sandbox case.
- Apple Development: the same entitlements, runtime flag set. It matches the hand-built "both" case, signed with a team.

| Signature | Helper loads | Second `dlopen` | Socket |
| --- | --- | --- | --- |
| Ad hoc, plain | yes | yes | yes |
| Ad hoc, sandbox | yes | yes | yes |
| Ad hoc, hardened runtime | no | no | no |
| Ad hoc, both | no | no | no |
| Team, both, ad hoc helper | no | no | no |
| Team, both, helper signed with the same identity | yes | yes | yes |

In the two ad hoc hardened rows dyld ignores `DYLD_INSERT_LIBRARIES`; Xcode never builds that signature.
In the team row AMFI logs `Library Validation failed ... different Team IDs` for the ad hoc helper.
The Xcode-built Debug app behaves as its hand-built match, in both signatures.

The condition: when the app carries a Team ID, Chord signs the helper and every reload dylib with the identity that signed the app.
`codesign -s <identity>` did this without a keychain prompt.
The alternative, `com.apple.security.cs.disable-library-validation`, is a setting in the user's project, which section 3.2 of `docs/ios.md` forbids.

The sandbox blocks none of it: with `sandbox_check` returning 1 and `~/Documents` denied, the app loaded and `dlopen`ed dylibs from `/tmp` and from another bundle id's folder in `~/Library/Caches`, two seconds after launch too.
The dylibs can stay in Chord's cache folder.

## 5. Decisions

1. **Chord signs what it loads into a team-signed app.** A Debug build signed with a Team ID loads only code that team signed (section 4, Q3). Chord reads the identity from the built app's signature and signs the helper and every reload dylib with it. An ad hoc build needs no signing. The user sets nothing in their project.
2. **The preview takes input**, as far as the input spike allows (section 6).
3. **The window shows at its own size.** The view scales it down when the panel is smaller and never scales it up.
4. **One window per preview process.** The helper builds one `NSWindow` at launch and swaps its content on a pick or a reload, so the context id reaches Chord once per launch.
5. **The preview process lives as the iOS preview app does.** It stays up while the worktree's preview is launched and ends with a rebuild, the worktree's delete and quit. No limit like the two booted preview simulators: a process costs no simulator.
6. **The Mac helper talks over an inherited socket.** Chord starts the process itself, so it passes one end of a `socketpair` at spawn. The iOS helper keeps its socket file, because `simctl launch` passes no descriptor. Both carry the same requests, one JSON line each way.
7. **The window sits on the screen Chord's window is on**, invisible, as I1 set it up. It takes its scale from that screen, and the helper moves it when Chord's window changes screens.
8. **The first version hosts no popover, menu, sheet or alert.** I3 failed for every kind, and each showed on the real screen. Section 8 holds the open question.
9. **A text field takes keys and shows no caret** (I2).

## 6. The input spike

It extends the two throwaway programs of Q2 and decides how far decision 2 reaches.
Checks I1, I2 and I4 read lines the agent prints, so they need no eye. I3 and I5 need one look at the screen.

### I1. Does the pointer reach a window that is off the screen?

The agent's view gains a `Button`, a `Slider`, a `ScrollView` and an `.onHover` area. Each prints a line when it reacts.
The agent builds `NSEvent`s and hands them to `window.sendEvent`: mouse down and up, a drag, mouse moved, and a scroll made from a `CGEvent`.

Pass: all four print.
Fail on hover or scroll alone: the first version ships without that one.
Fail on the click: decision 2 falls, and the preview stays a live picture.

### I2. Does a text field take keys?

The view gains a `TextField` that prints its text on each change.
Try in this order, and stop at the first that passes:

1. a click event, then key events to `window.sendEvent`;
2. the same, with `makeFirstResponder` called on the field's editor;
3. the same, in an `NSWindow` subclass that reports itself as the key window.

The host prints its own `isKeyWindow` after each try.

Pass: the typed text prints and the host's window stays key.
Fail: the first version takes the pointer and no keys.

Record whether the caret shows and whether the title bar turns active.

### I3. Where do the other windows go?

The view gains a popover, a `Menu`, a context menu, a sheet and an alert, each opened by an I1 click.
For each, record two facts: whether anything shows on the real screen, and whether `CAContext.allContexts()` gains a context the host can show.

Pass for a kind: nothing reaches the screen and the host shows its context.
Fail for a kind: the first version leaves that kind out. If the kind shows on the real screen, the build plan needs a way to keep it off, and the spike records which call would.

### I4. What does signing cost a reload?

Sign one reload-sized dylib ten times with the Apple Development identity and ten times ad hoc, and record both medians.
Record how to name the identity from the app alone: the fields of `codesign -d --verbose=4` that pick one certificate when the keychain holds two with one name.

Pass: the median stays under a tenth of the reload it joins. `docs/ios.md` has no reload time yet, so measure one iOS reload on the test project first.
Fail: sign in the background of the compile, or measure `codesign` against a linker-signed dylib re-signed in place, and record which helps.

### I5. Does the hosted window show under a webview?

The host puts a `WKWebView` that draws no background over the view that holds the `CALayerHost`, as `screen_view.rs` sits under Chord's webview.

Pass: the window shows through, by eye, and follows a resize of the host.
Fail: the Mac window needs a view above the webview, which changes step 5 of the build plan.

### Results

All runs: Xcode 27.1 (27A9275), macOS 27.0 (26A428), arm64.
The agent sent every event to itself; no event came from the host.
I3 and I5 got their look from the user, with the host on a second display.

#### I1. Pass, with the window ordered in

A window never ordered in takes no mouse event: `sendEvent` drops a mouse down before any hit test, for a plain `NSView` too.
A window ordered in at a frame outside every screen takes clicks, but has no `screen`, and a scroll then moves nothing.

The setup that passed, checked by eye in the host:

1. `setActivationPolicy(.accessory)`, the window built with `defer: false`.
2. `alphaValue = 0`, `ignoresMouseEvents = true`, then `orderFrontRegardless()` at a frame on a real screen.
3. The context id stays the one Q2 found, the counter ticks, and nothing shows on the screen or in Mission Control.

The events each control needs, all sent to `window.sendEvent` unless the line says otherwise:

- Click: `NSEvent.mouseEvent` down and up in window points, sharing one `eventNumber`.
- Drag: down, `leftMouseDragged` steps, up. The slider followed.
- Scroll: a pixel `CGEvent(scrollWheelEvent2Source:)` whose location holds the window point, flipped against the main screen's height, wrapped in `NSEvent(cgEvent:)`. The event carries no window, so AppKit reads its location as a window point.
- Hover: a `mouseEntered` from `NSEvent.enterExitEvent` whose `trackingNumber` is the hosting view's `NSTrackingArea`, then each move as a `mouseMoved` called on the hosting view itself. A `mouseMoved` sent to the window reaches no view.

SwiftUI's `.global` frames start at the top of the window, title bar included.

#### I2. Pass, way 1

A click on the field, then `keyDown` and `keyUp` events to `window.sendEvent`, typed the text.
The host's window stayed key.
The caret does not show (`_NSTextSelectionView` stays hidden, and the user saw none), and the title bar stays inactive.

#### I3. Fail for every kind

| Kind | Window in the agent | On the real screen | Host shows its context |
| --- | --- | --- | --- |
| Popover | `_NSPopoverWindow` | yes | yes |
| `Menu` | `NSPopupMenuWindow` | yes | yes |
| Alert | `_NSAlertPanel` | yes | yes |
| Sheet | `SheetPresentationWindow` | yes | no, only the dimmed window behind it |
| Context menu | none | no | unknown |

The menu's tracking takes the key window from the host while it runs.
A right mouse down built with `NSEvent.mouseEvent` fires the background's tap gesture and opens no context menu. `menu(for:)` on the hosting view and `NSMenu.popUpContextMenu` open it, with no window in the agent.

Two calls failed to keep a window off the screen, both made on `_NSWindowWillBecomeVisible`:

- `alphaValue = 0`: AppKit and the window list report alpha 0, and the user still saw all four kinds.
- `setFrameOrigin` outside every screen: the popover and the menu stay there by the window list, and the user still saw both. The alert and the sheet stay at their parent.

The spike found no call that keeps them off, and did not find out what the user saw: the window, a frame of it before the call, or an effect drawn apart from the window.

#### I4. Pass

The reload: `BreathingView.swift` in the Tuppo project, run by hand as `hot_reload.rs` runs it. The driver's `-###` took 0.39 s, the compile 0.62 s (median of five, each a cache miss), the link 0.03 s: 1.04 s.
The project caches compiles, so a save reruns the driver; without it the reload takes 0.65 s.

Signing that 670 KB dylib ten times each:

- Apple Development identity: median 66.7 ms.
- Ad hoc: median 16.1 ms.

66.7 ms stays under a tenth of 1.04 s, and lands 2 ms over a tenth of 0.65 s.
No keychain prompt showed.

No field of `codesign -d --verbose=4` picks one certificate: `Authority` holds the name, and this keychain holds several valid certificates with one name.
The SHA-1 of `cert0` from `codesign -d --extract-certificates` equals the hash `codesign -s` takes.
The ID in the certificate's name (`6KB4RD39RT`) differs from `TeamIdentifier` (`2V54FBNXT7`).

#### I5. Pass

The window showed under a `WKWebView` that draws no background, beside the webview's own text, and followed a resize of the host.

## 7. Build plan

Each step lands working code. Steps 1 to 5 give a live picture that follows saves, step 6 adds input.

### Step 1. Split the helper

`ChordPreview.swift` holds two jobs today: finding and capturing a `#Preview`, and showing it with UIKit.

- The registry scan, the capture, the layout trait and the request server move to a file both platforms compile.
- The UIKit part stays as the iOS shell.
- A new AppKit shell interposes the SwiftUI, `NSViewController` and `NSView` initializers (names in section 4, Q1), builds the one window of decision 4, sets `.accessory` and exports `__previews_injection_jit_link_entrypoint`.
- `helper()` in `preview.rs` takes the platform: the SDK, the target triple and the shell file. The cache folder's name gains the platform.

The Mac shell's replies to `show` and `load` carry the window's size in points and its scale. The reply to the first request carries the context id.

### Step 2. Signing

A new module, `signature.rs`, with two functions: the identity of a built app (none for ad hoc), and signing one file with it.
The identity is the SHA-1 of `cert0` from `codesign -d --extract-certificates`, the one value that picks a single certificate (I4).

- `helper()` signs its dylib and adds the identity to the cache folder's name, so two teams never share a helper.
- `link` in `hot_reload.rs` signs each reload dylib before `preview::load`.
- The iOS path calls neither: a simulator app runs without library validation.
- A `codesign` failure reaches the Preview view as `PreviewState::Failed` with the tool's first error line.

### Step 3. Launch and session

- `preview.rs` gains the Mac launch: the executable in `Contents/MacOS`, in a process group of its own as `mac.rs` starts a run, with `DYLD_INSERT_LIBRARIES` and the socket's descriptor number in the environment. It checks for `Contents/MacOS/<executable>.debug.dylib` and fails with the message the iOS launch gives.
- `request` takes its stream from the worktree's inherited socket when the preview runs on the Mac, and from the socket file otherwise.
- `Preview` in `preview_session.rs` holds `udid` and `boot` today. They move into a variant for the simulator, beside a Mac variant with neither.
- `open_preview` drops the failure for `MacSupport::Only`. A pure function picks the platform: `Only` gives the Mac, `Also` gives the Mac when the project's preview model is `RunDestination::Mac`, `None` gives the simulator.
- `preview_outcome` builds with `platform=macOS,arch=<arch>` for the Mac, as section 4a of `docs/ios.md` does for a run, then launches. No boot, no install, no `Full` state.
- The process ends where the iOS preview app ends: a rebuild, `forget_preview`, quit.

### Step 4. Hot reload

- `DriverLines::push` (`compile_commands.rs:61`) keeps a line whose target is `arm64-apple-macos*` too.
- `drivers.json` keys a driver by module today. A multiplatform module prints one line per platform, so the key becomes module and platform, and `reload` asks for the launched preview's platform.
- `link` already takes `-target` and `-sdk` from the compile command, so it needs the signing call of step 2 and nothing else.
- `worktree_saves.rs` and `view_graph.rs` stay as they are.

### Step 5. The window in the Preview view

- `Screen::create` and `prepare_screen` fail without SimulatorKit. A Mac-only project may run on a machine with no iOS platform, so the Mac window must not load it.
- `show_screen` takes `preview: bool` today. It takes what to show instead: the run simulator, the preview simulator, or a Mac preview with its context id and size.
- For a Mac preview the view holds a `CALayerHost` and a shadow layer: no plate, no bezel, no button capsule, no frame callbacks.
- A pure function gives the window's scale in the panel: 1 when it fits, the ratio that fits it otherwise, never above 1.
- A new size in a `show` or `load` reply moves the layer.
- The frontend's `Bezel` and `ScreenRect` reach the Mac window through the same `move_screen`.

### Step 6. Input

Only the parts the input spike passed.

- The Simulator view's pointer and key events reach Rust through `send_input` today. For a Mac preview Rust sends them to the helper as an `input` request, in window points, after undoing the scale of step 5.
- The helper turns each into the event section 6, I1 and I2 record.
- Hover lands last, as its own change: it needs the hosting view's tracking area and a direct `mouseMoved` call.
- Points count from the top of the window, title bar included.
- No other window passed I3: the first version takes no popover, menu, context menu, sheet or alert.

### Step 7. The Preview view and its design file

- `design/preview-view.md`: the Screen entry gains the Mac window, the Header entry the Mac's icon and name.
- The model menu lists My Mac for a project that also builds for iOS. A Mac-only project shows the name with no menu.
- `previewLine` keeps its texts; `full` and `starting` never arrive from a Mac preview.

### Step 8. `docs/ios.md`

Section 3.3 (the preview's slot holds for the Mac), section 4a (previews), section 5 (the preview process costs no simulator) and the code list in section 6.

### Tests

Three, each in a pure module with hardcoded values: the driver line kept for a macOS target, the platform a project and a preview model give, the window's scale in a panel.

### Proof

On a real Mac project, in both signatures: open a preview, save a view file, and read the helper's reply to `load`. The picture needs the user's eye (section 4, Q2).

## 8. Not verified

- Hot reload on macOS from the driver line to the helper's `load`. The first spike loaded a hand-built dylib.
- A second `#Preview` in the one window of decision 4, and a preview of an `NSViewController`.
- A window built with `defer: true`, and the scale when the Chord window moves between a Retina and a non-Retina display.
- A keychain prompt for `codesign` on another machine. The spike's machine showed none.
- A preview in a Swift package target with no app in its scheme.
- Input from Chord's view: the input spike's agent sent every event to itself.
- What shows on the screen when the preview opens a popover, menu, sheet or alert (section 6, I3). The host showed their contexts in the same run, so one run with the host showing only the main window tells a stray window from the host's own picture.
- A `#Preview` that presents a sheet, popover or alert as it appears: it opens that window with no click from the user.
- A menu's tracking takes the key window from Chord while it runs (I3).
