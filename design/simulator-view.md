# Simulator view

- **Screen:** the worktree's simulator, live, as large as fits with its bezel and its buttons 12px inside the view, keeping its shape; the screen and the buttons under it center as one group.
  The toolbar's phone button names the view.
  Its corners round by 12% of its width.
  Chord draws it natively under the window's web content, which opens a hole over it, so a toast, a menu or the scrim paints over the screen.
  A sheet or Settings closes the hole: their blur cannot reach the screen, and the canvas shows in its place until they close.
- **Buttons:** one rail centered 16px under the bezel: a 32px pill with a 1px `line-strong` edge holding Home, Lock and Shake, then the screenshot and record buttons, then the Appearance button, icons only, each button 36 × 28px and round with a 16px icon.
  A 1px × 16px `line-strong` hairline with 4px on each side splits the three groups.
  A view too narrow for the rail, 12px clear of its edges, wraps it: the hairlines go and each group takes its own 32px pill with the same edge, the later groups on centered lines 8px under the first, and the screen shrinks to keep the group in the view.
  Each button sits 1px inside the edge, so its fill never touches the hairline.
  Each button shows an `ink-secondary` icon; tooltips "Home (⌘⇧H)", "Lock (⌘L)", "Shake (⌃⌘Z)", "Take Screenshot (⌘S)", "Record Screen" (while it records "Stop Recording"), "Appearance".
  Each takes a `raised` pill on hover and `selected` when pressed, with its icon at 90%; the focus ring sits inside the button.
  They show only with the screen.
  When the lock icon turns between Lock and Unlock, its shackle closes or opens over 300ms and the icon tips 8° and back over 350ms, both easing out.
  A Shake rocks the phone icon 15° side to side, dying out over 450ms.
  Chord draws them natively with the screen, so they never slide against it on a resize.
- **Appearance:** the last button in the rail (half-filled circle icon).
  It opens a native menu: Light and Dark (the current one checked), then the submenus Text Size (the Dynamic Type sizes, extra small to accessibility extra extra extra large), Language and Region, each with the current choice checked and named in the Mac's language.
  Light, Dark and the text size change the simulator at once; a language or region applies to an app the next time it launches.
  The Preview view has it too, for its preview simulator.
- **Input:** a click or drag on the screen is a touch; a drag that leaves the screen keeps touching its edge, and one that starts on the bottom 3% is a swipe from the bottom edge.
  A trackpad pinch or rotate over the screen is a two-finger pinch or rotate.
  ⌥-drag pinches and rotates around the screen's center, ⌥⇧-drag moves two fingers together.
  While ⌥ is held over the screen, two 22px circles (`sheet` at 60%, 1px `line-strong` border) show where the fingers go down.
- **Bezel:** an 8px `bezel` frame around the screen, its corners following the screen's, with a 1px `line-strong` edge outside it, as a phone's.
  Chord draws it natively with the screen, so the frame never slides against the screen on a resize.
  It shows the same with or without the focus: no focus ring.
- **Screenshot:** the camera button in the rail takes a shot.
  A shot leaves a thumbnail at the view's left, 8px from its edge, its bottom level with the button row's bottom: the screenshot at most 56px wide and 96px tall, on `sheet` with a 1px `line-strong` edge, Inset radius and the popover shadow; it fades in and rises 4px over 200ms with `--ease-out`, and fades out and drops 4px over 200ms with a plain ease-in when it goes; under Reduce Motion it only fades.
  It goes after 4 seconds unless the pointer is over it, when a newer shot replaces it, or when a click pastes it.
  Dragging it out is a file drag: Finder, any app and a chat pane that takes attachments all take it, the pane as the Chat section's image drop.
  A click attaches it to the focused agent's next message, if that agent takes attachments, and focuses its composer.
- **Recording:** the record button (a filled dot) starts a screen recording of the simulator.
  On the click the rail grows the recording's time in before the button over 200ms with `--ease-out`, the row staying centered: the recording's time ("0:02", m:ss, 13px/500 `ink`, tabular, 12px after the hairline, 6px before the button), then the button as a `danger` square on a `danger` wash at 18% (26% on hover, 34% pressed).
  The time reads 0:00 until the simulator records and stops counting on the stop click; a click on the button stops the recording.
  When the recording ends the time folds away and the button shows its dot.
  Under Reduce Motion the time shows and goes at once.
  The stopped video leaves the screenshot's thumbnail, its first frame, which drags out as a file the same way.
  A click on it does nothing: no agent takes a video.
  A recording that ends by itself, as when the simulator shuts down, leaves its thumbnail too and the button shows the dot again.
  A recording stops when Chord quits or crashes, so the file is complete.
- **Focus:** a click on the screen gives it the focus.
  While it has the focus every key goes to the simulator, and ⌘ keys stay the window's shortcuts.
- **Off:** with no screen, a centered meta line at 40% of the view's height: "The simulator is off." and under it, 12px below, a secondary "Run" button (tooltip "Run (⌘R)", disabled while a build starts) that runs as ⌘R does; with a device or My Mac as the run destination "Runs go to <device or My Mac>. Pick a simulator in Run Destination to run here." and no button; the simulator's own note while it starts or restarts, or, when Xcode's private frameworks do not load, their message and "Run (⌘R) opens DeviceHub instead." with the selected Xcode's simulator app, and under the line a secondary button "Open in <app>", DeviceHub or Simulator (the same app, 12px below the line) that opens it.
