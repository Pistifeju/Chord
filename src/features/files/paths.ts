// Paths in a worktree: the name a path ends in, the folder that holds a relative path, a name joined to a folder, and where a path lands after a rename.

/** The last part of `path`: a file or folder name. */
export const fileName = (path: string) => path.slice(path.lastIndexOf("/") + 1);

/** The folder that holds `path`; "" for the worktree root. */
export const parentFolder = (path: string) => path.slice(0, Math.max(path.lastIndexOf("/"), 0));

export const joinPath = (folder: string, name: string) => (folder ? `${folder}/${name}` : name);

/** `path` is `at` or lies inside the folder `at`. */
export const isAtOrInside = (path: string, at: string) => path === at || path.startsWith(`${at}/`);

/** Where `path` is after a rename of `from` to `to`. */
export const movedPath = (path: string, from: string, to: string) => (isAtOrInside(path, from) ? to + path.slice(from.length) : path);
