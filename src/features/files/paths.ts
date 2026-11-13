// Paths relative to a worktree's folder: the folder that holds one, a name joined to a folder, and where a path lands after a rename.

/** The folder that holds `path`; "" for the worktree root. */
export const parentFolder = (path: string) => path.slice(0, Math.max(path.lastIndexOf("/"), 0));

export const joinPath = (folder: string, name: string) => (folder ? `${folder}/${name}` : name);

/** `path` is `at` or lies inside the folder `at`. */
export const isAtOrInside = (path: string, at: string) => path === at || path.startsWith(`${at}/`);

/** Where `path` is after a rename of `from` to `to`. */
export const movedPath = (path: string, from: string, to: string) => (isAtOrInside(path, from) ? to + path.slice(from.length) : path);
