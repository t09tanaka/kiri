/**
 * CodeMirror extension for displaying Git diff markers in the editor
 *
 * Features:
 * - Gutter markers (colored bars) for added/modified/deleted lines
 * - Line background colors for added/modified lines
 */

import { EditorView, Decoration, type DecorationSet, GutterMarker, gutter } from '@codemirror/view';
import {
  StateField,
  StateEffect,
  RangeSetBuilder,
  RangeSet,
  type Extension,
} from '@codemirror/state';
import { type ParsedDiff, parseDiff } from '@/lib/utils/diffParser';

/**
 * Effect to set git diff data
 */
export const setGitDiffEffect = StateEffect.define<string>();

/**
 * Gutter marker for added lines
 */
class AddedMarker extends GutterMarker {
  toDOM() {
    const marker = document.createElement('div');
    marker.className = 'cm-git-gutter-added';
    return marker;
  }
}

/**
 * Gutter marker for modified lines
 */
class ModifiedMarker extends GutterMarker {
  toDOM() {
    const marker = document.createElement('div');
    marker.className = 'cm-git-gutter-modified';
    return marker;
  }
}

/**
 * Gutter marker for deleted lines (shows where deletion occurred)
 */
class DeletedMarker extends GutterMarker {
  toDOM() {
    const marker = document.createElement('div');
    marker.className = 'cm-git-gutter-deleted';
    return marker;
  }
}

// Lazy-initialized marker instances
let addedMarker: AddedMarker | null = null;
let modifiedMarker: ModifiedMarker | null = null;
let deletedMarker: DeletedMarker | null = null;

function getAddedMarker(): AddedMarker {
  if (!addedMarker) addedMarker = new AddedMarker();
  return addedMarker;
}

function getModifiedMarker(): ModifiedMarker {
  if (!modifiedMarker) modifiedMarker = new ModifiedMarker();
  return modifiedMarker;
}

function getDeletedMarker(): DeletedMarker {
  if (!deletedMarker) deletedMarker = new DeletedMarker();
  return deletedMarker;
}

/**
 * State field to store parsed diff data and gutter markers
 */
const gitDiffStateField = StateField.define<{
  parsed: ParsedDiff;
  markers: RangeSet<GutterMarker>;
}>({
  create() {
    return {
      parsed: { addedLines: [], modifiedLines: [], deletedAtLines: [] },
      markers: RangeSet.empty,
    };
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setGitDiffEffect)) {
        const parsed = parseDiff(effect.value);
        const markers = buildGutterMarkers(parsed, tr.state.doc);
        return { parsed, markers };
      }
    }
    // Map markers through document changes
    if (tr.docChanged) {
      return {
        parsed: value.parsed,
        markers: value.markers.map(tr.changes),
      };
    }
    return value;
  },
});

/**
 * Build gutter markers from parsed diff
 */
function buildGutterMarkers(
  parsed: ParsedDiff,
  doc: { lines: number; line: (n: number) => { from: number } }
): RangeSet<GutterMarker> {
  const builder = new RangeSetBuilder<GutterMarker>();
  const markers: Array<{ pos: number; marker: GutterMarker }> = [];

  for (const lineNum of parsed.addedLines) {
    if (lineNum > 0 && lineNum <= doc.lines) {
      const line = doc.line(lineNum);
      markers.push({ pos: line.from, marker: getAddedMarker() });
    }
  }

  for (const lineNum of parsed.modifiedLines) {
    if (lineNum > 0 && lineNum <= doc.lines) {
      const line = doc.line(lineNum);
      markers.push({ pos: line.from, marker: getModifiedMarker() });
    }
  }

  for (const lineNum of parsed.deletedAtLines) {
    if (lineNum > 0 && lineNum <= doc.lines) {
      const line = doc.line(lineNum);
      markers.push({ pos: line.from, marker: getDeletedMarker() });
    }
  }

  // Sort by position (required by RangeSetBuilder)
  markers.sort((a, b) => a.pos - b.pos);

  for (const { pos, marker } of markers) {
    builder.add(pos, pos, marker);
  }

  return builder.finish();
}

/**
 * Line decoration for added lines
 */
const addedLineDecoration = Decoration.line({
  class: 'cm-git-line-added',
});

/**
 * Line decoration for modified lines
 */
const modifiedLineDecoration = Decoration.line({
  class: 'cm-git-line-modified',
});

/**
 * State field for line decorations (background colors)
 */
const gitDiffDecorationsField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(decorations, tr) {
    // Check if diff data changed
    for (const effect of tr.effects) {
      if (effect.is(setGitDiffEffect)) {
        const parsed = parseDiff(effect.value);
        const builder = new RangeSetBuilder<Decoration>();

        // Collect all decorated lines and sort by position
        const decoratedLines: Array<{
          lineNum: number;
          decoration: Decoration;
        }> = [];

        for (const lineNum of parsed.addedLines) {
          if (lineNum > 0 && lineNum <= tr.state.doc.lines) {
            decoratedLines.push({ lineNum, decoration: addedLineDecoration });
          }
        }

        for (const lineNum of parsed.modifiedLines) {
          if (lineNum > 0 && lineNum <= tr.state.doc.lines) {
            decoratedLines.push({ lineNum, decoration: modifiedLineDecoration });
          }
        }

        // Sort by line number to maintain order (required by RangeSetBuilder)
        decoratedLines.sort((a, b) => a.lineNum - b.lineNum);

        for (const { lineNum, decoration } of decoratedLines) {
          const line = tr.state.doc.line(lineNum);
          builder.add(line.from, line.from, decoration);
        }

        return builder.finish();
      }
    }

    // Map decorations through document changes
    if (tr.docChanged) {
      return decorations.map(tr.changes);
    }

    return decorations;
  },
  provide: (field) => EditorView.decorations.from(field),
});

/**
 * Creates the git diff extension for CodeMirror
 * Returns an array of extensions that should be added to the editor
 */
export function gitDiffExtension(): Extension[] {
  const gitDiffGutter = gutter({
    class: 'cm-git-gutter',
    markers: (view) => {
      const field = view.state.field(gitDiffStateField, false);
      return field ? field.markers : RangeSet.empty;
    },
  });

  const gitDiffTheme = EditorView.baseTheme({
    '.cm-git-gutter': {
      width: '4px',
      marginRight: '2px',
    },
    '.cm-git-gutter-added': {
      backgroundColor: 'var(--git-added)',
      width: '3px',
      height: '100%',
      borderRadius: '1px',
    },
    '.cm-git-gutter-modified': {
      backgroundColor: 'var(--git-modified)',
      width: '3px',
      height: '100%',
      borderRadius: '1px',
    },
    '.cm-git-gutter-deleted': {
      backgroundColor: 'var(--git-deleted)',
      width: '8px',
      height: '2px',
      marginTop: 'calc(50% - 1px)',
      borderRadius: '1px',
    },
    '.cm-git-line-added': {
      backgroundColor: 'rgba(74, 222, 128, 0.06)',
    },
    '.cm-git-line-modified': {
      backgroundColor: 'rgba(251, 191, 36, 0.06)',
    },
  });

  return [gitDiffStateField, gitDiffDecorationsField, gitDiffGutter, gitDiffTheme];
}

/**
 * Helper function to update git diff in an editor view
 */
export function updateGitDiff(view: EditorView, diffContent: string) {
  view.dispatch({
    effects: setGitDiffEffect.of(diffContent),
  });
}

/**
 * Clear git diff decorations
 */
export function clearGitDiff(view: EditorView) {
  view.dispatch({
    effects: setGitDiffEffect.of(''),
  });
}
