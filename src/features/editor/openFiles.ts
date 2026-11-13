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

/** `path` is `at` or lies inside the folder `at`. */
export const isAtOrInside = (path: string, at: string) => path === at || path.startsWith(`${at}/`);

/** Where `path` is after a rename of `from` to `to`. */
export const movedPath = (path: string, from: string, to: string) => (isAtOrInside(path, from) ? to + path.slice(from.length) : path);

// A rename keeps the tabs of the entry and of everything inside it, under their new paths.
export const withFilesMoved = (open: OpenFiles, from: string, to: string): OpenFiles => ({
  files: open.files.map((path) => movedPath(path, from, to)),
  active: open.active && movedPath(open.active, from, to),
});
