let opening = false;

export async function openContextMenu(open: () => Promise<void>) {
  if (opening) return;
  opening = true;
  try {
    await open();
  } finally {
    opening = false;
  }
}
