import {
  detectEmbeddedContext,
  escapeHtml,
  getLanguageFromPath,
  getLineLanguage,
  highlightLine,
  supportsEmbeddedLanguages,
  type EmbeddedContext,
} from '@/lib/utils/syntaxHighlight';

export interface DiffLine {
  type: 'add' | 'remove' | 'context' | 'header';
  content: string;
  highlightedContent: string;
  lineNumber: number | null;
}

export interface DiffStats {
  additions: number;
  deletions: number;
}

/**
 * Parse the textual diff for a single file into syntax-highlighted lines.
 * Inputs follow git-style markers: `+ ` / `- ` / `  ` / `@@ ...`.
 *
 * The parser tracks `lineNum` from the hunk header so added/context lines
 * carry their post-image line number for the gutter. For files that
 * embed multiple languages (e.g. .vue, .svelte) the parser sniffs the
 * upcoming lines after each hunk header so highlight rules can switch
 * between template / script / style sections.
 */
export function parseDiff(path: string, diffContent: string): DiffLine[] {
  if (!diffContent) return [];

  const baseLanguage = getLanguageFromPath(path);
  const hasEmbedded = supportsEmbeddedLanguages(path);
  const lines: DiffLine[] = [];
  let lineNum = 0;
  let context: EmbeddedContext = 'template';

  const rawLines = diffContent.split('\n');

  for (let i = 0; i < rawLines.length; i++) {
    const line = rawLines[i];
    let content: string;
    let type: DiffLine['type'];
    let lineNumber: number | null;

    if (line.startsWith('+ ')) {
      lineNum++;
      content = line.slice(2);
      type = 'add';
      lineNumber = lineNum;
    } else if (line.startsWith('- ')) {
      content = line.slice(2);
      type = 'remove';
      lineNumber = null;
    } else if (line.startsWith('  ')) {
      lineNum++;
      content = line.slice(2);
      type = 'context';
      lineNumber = lineNum;
    } else if (line.startsWith('@@')) {
      const match = line.match(/@@ -\d+(?:,\d+)? \+(\d+)/);
      if (match) {
        lineNum = parseInt(match[1], 10) - 1;
      }

      if (hasEmbedded) {
        const upcomingContent: string[] = [];
        for (let j = i + 1; j < rawLines.length; j++) {
          const next = rawLines[j];
          if (next.startsWith('@@')) break;
          if (next.startsWith('+ ') || next.startsWith('- ') || next.startsWith('  ')) {
            upcomingContent.push(next.slice(2));
          }
          if (upcomingContent.length >= 10) break;
        }
        context = detectEmbeddedContext(upcomingContent);
      }

      lines.push({
        type: 'header',
        content: line,
        highlightedContent: escapeHtml(line),
        lineNumber: null,
      });
      continue;
    } else {
      continue;
    }

    let language = baseLanguage;
    if (hasEmbedded) {
      const result = getLineLanguage(content, baseLanguage, context);
      language = result.language;
      context = result.newContext;
    }

    lines.push({
      type,
      content,
      highlightedContent: highlightLine(content, language),
      lineNumber,
    });
  }

  return lines;
}

export function computeDiffStats(diffContent: string): DiffStats {
  let additions = 0;
  let deletions = 0;
  if (diffContent) {
    for (const line of diffContent.split('\n')) {
      if (line.startsWith('+ ')) {
        additions++;
      } else if (line.startsWith('- ')) {
        deletions++;
      }
    }
  }
  return { additions, deletions };
}

export function getFileName(path: string): string {
  return path.split('/').pop() ?? path;
}

export function getDiffId(path: string): string {
  return `diff-${path.replace(/[^a-zA-Z0-9]/g, '-')}`;
}

const IMAGE_MIME_TYPES: Record<string, string> = {
  png: 'png',
  jpg: 'jpeg',
  jpeg: 'jpeg',
  gif: 'gif',
  ico: 'x-icon',
  webp: 'webp',
  bmp: 'bmp',
  svg: 'svg+xml',
  tiff: 'tiff',
  tif: 'tiff',
};

export function getImageMimeType(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  return IMAGE_MIME_TYPES[ext] ?? 'png';
}

/**
 * Conservative line count for the lazy-load placeholder height. Real
 * diffs usually render shorter once context is collapsed, but this gives
 * enough scroll room to defer rendering until intersection.
 */
export function estimateLineCount(diff: string): number {
  if (!diff) return 3;
  return Math.max(3, diff.split('\n').length);
}
