export function shellPath(path: string): string {
  return /^[\w@%+=:,./-]+$/.test(path) ? path : `'${path.replace(/'/g, "'\\''")}'`;
}

export const droppedPathsText = (paths: string[]) => `${paths.map(shellPath).join(" ")} `;
