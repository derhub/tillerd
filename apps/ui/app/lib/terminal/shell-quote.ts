// POSIX single-quote a filesystem path so it reaches the shell as one literal argument.
// Single quotes suppress every expansion; the only character that cannot appear inside them is a
// single quote itself, closed and reopened around an escaped one ('\'').
export function shellQuotePath(path: string): string {
  return `'${path.replaceAll("'", "'\\''")}'`;
}
