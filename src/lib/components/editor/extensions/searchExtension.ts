import {
  search,
  highlightSelectionMatches,
  searchPanelOpen,
  openSearchPanel,
  closeSearchPanel,
  getSearchQuery,
} from '@codemirror/search';
import { EditorView, ViewPlugin, keymap, type ViewUpdate } from '@codemirror/view';
import { EditorState, type Extension } from '@codemirror/state';

export { searchPanelOpen };

/**
 * Custom keybindings for search functionality.
 * Excludes Mod-g (Worktree toggle) and Mod-d (DiffView toggle) to avoid conflicts.
 */
const searchKeymap = keymap.of([
  { key: 'Mod-f', run: openSearchPanel, scope: 'editor search-panel' },
  { key: 'Escape', run: closeSearchPanel, scope: 'editor search-panel' },
]);

/**
 * Mist theme styles for the search panel and match highlights.
 * Designed to blend with kiri's "Mist & Tranquility" aesthetic.
 */
const searchTheme = EditorView.theme(
  {
    // Search panel container
    '.cm-panel.cm-search': {
      background: 'rgba(13, 17, 23, 0.85)',
      backdropFilter: 'blur(16px)',
      WebkitBackdropFilter: 'blur(16px)',
      borderBottom: '1px solid rgba(125, 211, 252, 0.15)',
      padding: '8px',
      gap: '6px',
      display: 'flex',
      alignItems: 'center',
      fontFamily: "'IBM Plex Mono', 'JetBrains Mono', 'SF Mono', monospace",
      fontSize: '12px',
      color: '#c8d3e0',
    },

    // Hide match case, regexp, by word checkboxes
    '.cm-panel.cm-search label': {
      display: 'none',
    },

    '.cm-panel.cm-search [name=select]': {
      display: 'none',
    },

    // Hide next/previous buttons
    '.cm-panel.cm-search button[name=next], .cm-panel.cm-search button[name=prev]': {
      display: 'none',
    },

    // Match count indicator (e.g. "1/3")
    '.cm-search-count': {
      fontSize: '11px',
      color: '#8b99a8',
      marginLeft: '8px',
      whiteSpace: 'nowrap' as const,
    },

    '.cm-search-count.no-results': {
      color: '#5c6b7a',
    },

    // Search input fields
    '.cm-panel.cm-search input, .cm-panel.cm-search button:not(.cm-button)': {
      fontFamily: "'IBM Plex Mono', 'JetBrains Mono', 'SF Mono', monospace",
      fontSize: '12px',
    },

    '.cm-panel.cm-search input': {
      flex: '1',
      background: 'rgba(20, 27, 35, 0.8)',
      border: '1px solid rgba(125, 211, 252, 0.15)',
      borderRadius: '6px',
      color: '#e6edf3',
      padding: '6px 10px',
      fontSize: '13px',
      outline: 'none',
      transition: 'border-color 180ms ease, box-shadow 180ms ease',
      margin: '0',
    },

    '.cm-panel.cm-search input:focus': {
      borderColor: 'rgba(125, 211, 252, 0.5)',
      boxShadow: '0 0 0 2px rgba(125, 211, 252, 0.1)',
    },

    '.cm-panel.cm-search input::placeholder': {
      color: '#5c6b7a',
    },

    // Search buttons
    '.cm-panel.cm-search button': {
      background: 'transparent',
      border: '1px solid rgba(125, 211, 252, 0.12)',
      borderRadius: '6px',
      color: '#8b99a8',
      padding: '3px 10px',
      cursor: 'pointer',
      transition: 'all 180ms ease',
      fontSize: '11px',
      whiteSpace: 'nowrap' as const,
    },

    '.cm-panel.cm-search button:hover': {
      background: 'rgba(125, 211, 252, 0.1)',
      borderColor: 'rgba(125, 211, 252, 0.3)',
      color: '#7dd3fc',
    },

    '.cm-panel.cm-search button:active': {
      background: 'rgba(125, 211, 252, 0.15)',
    },

    // Close button - override CodeMirror's absolute positioning
    '.cm-panel.cm-search button[name=close]': {
      position: 'static',
      marginLeft: 'auto',
      background: 'transparent',
      border: 'none',
      color: '#5c6b7a',
      padding: '2px 6px',
      fontSize: '16px',
      lineHeight: '1',
      borderRadius: '4px',
    },

    '.cm-panel.cm-search button[name=close]:hover': {
      background: 'rgba(248, 113, 113, 0.1)',
      color: '#f87171',
    },

    // Search match highlights in the editor
    '.cm-searchMatch': {
      backgroundColor: 'rgba(125, 211, 252, 0.2)',
      outline: '1px solid rgba(125, 211, 252, 0.4)',
      borderRadius: '2px',
    },

    // Currently selected match
    '.cm-searchMatch-selected': {
      backgroundColor: 'rgba(196, 181, 253, 0.3)',
      outline: '1px solid rgba(196, 181, 253, 0.5)',
    },

    // Selection match highlights (other occurrences of selected text)
    '.cm-selectionMatch': {
      backgroundColor: 'rgba(125, 211, 252, 0.1)',
      borderRadius: '2px',
    },
  },
  { dark: true }
);

/**
 * ViewPlugin that displays match count (e.g. "1/3") in the search panel.
 * Inserts a counter element after the search input field.
 */
const matchCountPlugin = ViewPlugin.fromClass(
  class {
    countEl: HTMLElement | null = null;
    lastQuery = '';
    lastSelFrom = -1;

    update(update: ViewUpdate) {
      const view = update.view;
      if (!searchPanelOpen(view.state)) {
        this.countEl = null;
        this.lastQuery = '';
        return;
      }

      const query = getSearchQuery(view.state);
      const selFrom = view.state.selection.main.from;

      // Only recalculate when query or selection changes
      if (query.search === this.lastQuery && selFrom === this.lastSelFrom && this.countEl) {
        return;
      }
      this.lastQuery = query.search;
      this.lastSelFrom = selFrom;

      const panel = view.dom.closest('.cm-editor')?.querySelector('.cm-panel.cm-search');
      if (!panel) return;

      const input = panel.querySelector('input[name="search"]') as HTMLInputElement | null;

      // Ensure counter element exists
      if (!this.countEl || !panel.contains(this.countEl)) {
        this.countEl = document.createElement('span');
        this.countEl.className = 'cm-search-count';
        if (input) {
          input.after(this.countEl);
        }
      }

      if (!query.search) {
        this.countEl.textContent = '';
        this.countEl.classList.remove('no-results');
        return;
      }

      // Disable autocapitalize on search input
      if (input && !input.hasAttribute('autocapitalize')) {
        input.setAttribute('autocapitalize', 'off');
        input.setAttribute('autocorrect', 'off');
        input.setAttribute('spellcheck', 'false');
      }

      // Count matches using SearchCursor
      const cursor = query.getCursor(view.state.doc);
      let total = 0;
      let currentIdx = 0;
      let firstAfterCursor = 0;

      let result = cursor.next();
      while (!result.done) {
        total++;
        if (result.value.from === selFrom) {
          currentIdx = total;
        }
        if (firstAfterCursor === 0 && result.value.from >= selFrom) {
          firstAfterCursor = total;
        }
        result = cursor.next();
      }

      if (total === 0) {
        this.countEl.textContent = 'No results';
        this.countEl.classList.add('no-results');
      } else {
        const displayIdx =
          currentIdx > 0 ? currentIdx : firstAfterCursor > 0 ? firstAfterCursor : 1;
        this.countEl.textContent = `${displayIdx}/${total}`;
        this.countEl.classList.remove('no-results');
      }
    }

    destroy() {
      this.countEl?.remove();
    }
  }
);

/**
 * Creates search extensions for CodeMirror editor.
 * Provides in-file search with Mist-themed UI.
 */
export function searchExtension(): Extension[] {
  return [
    search({ top: true }),
    highlightSelectionMatches(),
    searchKeymap,
    searchTheme,
    matchCountPlugin,
    EditorState.readOnly.of(true),
  ];
}
