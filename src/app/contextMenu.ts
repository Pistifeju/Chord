// Opens the app's native pop-up menus one at a time.

let opening = false;

/** Runs `open`, which builds a menu and pops it up; a call while another is building or open does nothing. */
export async function openContextMenu(open: () => Promise<void>) {
  // Tauri holds the webview's resource lock while a menu is open, so building a second menu then deadlocks the main thread.
  if (opening) return;
  opening = true;
  try {
    await open();
  } finally {
    opening = false;
  }
}
