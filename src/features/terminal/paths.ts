// Rules for dropped paths in the terminal: how a path is written for the shell.

// Single quotes keep every character literal in sh, bash and zsh; a quote inside closes, escapes and reopens them.
export function shellPath(path: string): string {
  return /^[\w@%+=:,./-]+$/.test(path) ? path : `'${path.replace(/'/g, "'\\''")}'`;
}

// The trailing space lets the user type the next word right away.
export const droppedPathsText = (paths: string[]) => `${paths.map(shellPath).join(" ")} `;
