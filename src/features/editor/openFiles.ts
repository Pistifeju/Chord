// Rules for a worktree's file tabs: which files are open and which one the window shows.

export type OpenFiles = { files: string[]; active: string | null };

export const noFiles: OpenFiles = { files: [], active: null };

export const withFileOpened = (open: OpenFiles, path: string): OpenFiles => ({
  files: open.files.includes(path) ? open.files : [...open.files, path],
  active: path,
});

// Closing the shown file shows the tab beside it, or the sessions once no file is left.
export function withFileClosed(open: OpenFiles, path: string): OpenFiles {
  const at = open.files.indexOf(path);
  if (at < 0) return open;
  const files = open.files.filter((file) => file !== path);
  return { files, active: open.active === path ? (files[at] ?? files[at - 1] ?? null) : open.active };
}

export const withSessionsShown = (open: OpenFiles): OpenFiles => (open.active === null ? open : { ...open, active: null });
