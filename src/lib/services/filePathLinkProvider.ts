import type { Terminal, ILinkProvider, ILink } from '@xterm/xterm';

/**
 * File path pattern regex
 * Matches patterns like:
 * - src/lib/utils/date.ts:42
 * - ./components/App.svelte:10:5
 * - /absolute/path/file.rs:100
 * - file.ts
 */
const FILE_PATH_REGEX =
  /(?:^|[\s'"([\]])((\.{0,2}\/)?(?:[\w@.-]+\/)*[\w.-]+\.(?:ts|tsx|js|jsx|svelte|rs|md|json|toml|css|scss|html|yaml|yml|txt|sh|py|go|c|cpp|h|hpp|java|rb|php|vue|astro|prisma))(?::(\d+))?(?::(\d+))?(?=[\s'")\]:]|$)/g;

export interface FilePathMatch {
  path: string;
  line?: number;
  column?: number;
  startIndex: number;
  endIndex: number;
}

/**
 * Parse file paths from a line of text
 */
export function parseFilePathsFromLine(lineText: string): FilePathMatch[] {
  const matches: FilePathMatch[] = [];

  // Reset regex state
  FILE_PATH_REGEX.lastIndex = 0;

  let match;
  while ((match = FILE_PATH_REGEX.exec(lineText)) !== null) {
    const fullMatch = match[0];
    const path = match[1];
    const line = match[3] ? parseInt(match[3], 10) : undefined;
    const column = match[4] ? parseInt(match[4], 10) : undefined;

    // Calculate the actual start position of the path within the match
    // The match might include leading whitespace/quotes
    const pathStartInMatch = fullMatch.indexOf(path);
    const startIndex = match.index + pathStartInMatch;

    // Calculate end index (path + optional :line + optional :column)
    let pathWithLineCol = path;
    if (line !== undefined) {
      pathWithLineCol += `:${line}`;
      if (column !== undefined) {
        pathWithLineCol += `:${column}`;
      }
    }
    const endIndex = startIndex + pathWithLineCol.length;

    matches.push({
      path,
      line,
      column,
      startIndex,
      endIndex,
    });
  }

  return matches;
}

/**
 * Create a link provider for file paths in terminal output
 */
export function createFilePathLinkProvider(
  terminal: Terminal,
  onActivate: (path: string, line?: number, column?: number) => void
): ILinkProvider {
  return {
    provideLinks(bufferLineNumber: number, callback: (links: ILink[] | undefined) => void): void {
      const buffer = terminal.buffer.active;
      const line = buffer.getLine(bufferLineNumber - 1);

      if (!line) {
        callback(undefined);
        return;
      }

      const lineText = line.translateToString();
      const matches = parseFilePathsFromLine(lineText);

      if (matches.length === 0) {
        callback(undefined);
        return;
      }

      const links: ILink[] = matches.map((match) => ({
        range: {
          start: { x: match.startIndex + 1, y: bufferLineNumber }, // xterm is 1-indexed for x
          end: { x: match.endIndex + 1, y: bufferLineNumber },
        },
        text:
          match.path +
          (match.line ? `:${match.line}` : '') +
          (match.column ? `:${match.column}` : ''),
        activate: (_event: MouseEvent, _text: string) => {
          onActivate(match.path, match.line, match.column);
        },
        hover: (_event: MouseEvent, _text: string) => {
          // Optional: could add hover effects
        },
      }));

      callback(links);
    },
  };
}
