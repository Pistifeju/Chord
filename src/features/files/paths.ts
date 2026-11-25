export const fileName = (path: string) => path.slice(path.lastIndexOf("/") + 1);

export const parentFolder = (path: string) => path.slice(0, Math.max(path.lastIndexOf("/"), 0));

export const joinPath = (folder: string, name: string) => (folder ? `${folder}/${name}` : name);

export const isAtOrInside = (path: string, at: string) => path === at || path.startsWith(`${at}/`);

export const movedPath = (path: string, from: string, to: string) => (isAtOrInside(path, from) ? to + path.slice(from.length) : path);
