/**
 * Parser for unified diff format from git
 *
 * Diff format from Rust backend:
 * - Lines starting with "+ " are additions
 * - Lines starting with "- " are deletions
 * - Lines starting with "  " are context
 * - Lines starting with "@@" are hunk headers with line numbers
 */

export interface ParsedDiff {
  /** Line numbers of added lines (1-indexed, in current file) */
  addedLines: number[];
  /** Line numbers of modified lines (1-indexed, in current file) */
  modifiedLines: number[];
  /** Line numbers where deletions occurred (1-indexed, position in current file) */
  deletedAtLines: number[];
}

/**
 * Parse unified diff format to extract line change information
 *
 * Modified lines are detected when a deletion is immediately followed by an addition.
 * This is a heuristic since unified diff doesn't explicitly mark modifications.
 */
export function parseDiff(diffContent: string): ParsedDiff {
  const result: ParsedDiff = {
    addedLines: [],
    modifiedLines: [],
    deletedAtLines: [],
  };

  if (!diffContent) {
    return result;
  }

  const lines = diffContent.split('\n');
  let currentLineNumber = 0;
  let pendingDeletionCount = 0;
  let deletionStartLine = 0;

  for (const line of lines) {
    if (line.startsWith('@@')) {
      // Process any pending deletions before starting new hunk
      if (pendingDeletionCount > 0) {
        result.deletedAtLines.push(deletionStartLine);
        pendingDeletionCount = 0;
      }
      // Hunk header: @@ -oldStart,oldCount +newStart,newCount @@
      const match = line.match(/@@ -\d+(?:,\d+)? \+(\d+)/);
      if (match) {
        currentLineNumber = parseInt(match[1], 10) - 1;
      }
    } else if (line.startsWith('+ ')) {
      currentLineNumber++;
      if (pendingDeletionCount > 0) {
        // Previous deletion followed by addition = modification
        result.modifiedLines.push(currentLineNumber);
        pendingDeletionCount--;
      } else {
        result.addedLines.push(currentLineNumber);
      }
    } else if (line.startsWith('- ')) {
      // Track deletion at current position
      if (pendingDeletionCount === 0) {
        // Mark where the deletion block starts (next line position)
        deletionStartLine = currentLineNumber + 1;
      }
      pendingDeletionCount++;
      // Don't increment line number for deletions
    } else if (line.startsWith('  ')) {
      // Context line - process any pending deletions first
      if (pendingDeletionCount > 0) {
        // Deletions not followed by additions - pure deletions
        result.deletedAtLines.push(deletionStartLine);
        pendingDeletionCount = 0;
      }
      currentLineNumber++;
    }
  }

  // Handle any trailing deletions
  if (pendingDeletionCount > 0) {
    result.deletedAtLines.push(deletionStartLine);
  }

  return result;
}

/**
 * Get all line numbers that have any kind of change
 */
export function getAllChangedLines(parsed: ParsedDiff): Set<number> {
  const changedLines = new Set<number>();

  for (const line of parsed.addedLines) {
    changedLines.add(line);
  }
  for (const line of parsed.modifiedLines) {
    changedLines.add(line);
  }
  for (const line of parsed.deletedAtLines) {
    changedLines.add(line);
  }

  return changedLines;
}
