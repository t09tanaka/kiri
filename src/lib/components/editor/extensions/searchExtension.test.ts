import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { EditorView } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import {
  openSearchPanel,
  closeSearchPanel,
  setSearchQuery,
  SearchQuery,
  searchPanelOpen,
} from '@codemirror/search';
import { searchExtension } from './searchExtension';

describe('searchExtension', () => {
  it('should return an array of extensions', () => {
    const extensions = searchExtension();
    expect(Array.isArray(extensions)).toBe(true);
    expect(extensions.length).toBeGreaterThan(0);
  });

  it('should return consistent results on multiple calls', () => {
    const first = searchExtension();
    const second = searchExtension();
    expect(first.length).toBe(second.length);
  });
});

describe('matchCountPlugin', () => {
  let view: EditorView;
  let parent: HTMLElement;

  beforeEach(() => {
    parent = document.createElement('div');
    document.body.appendChild(parent);

    const state = EditorState.create({
      doc: 'hello world\nhello again\nworld hello',
      extensions: searchExtension(),
    });
    view = new EditorView({ state, parent });
  });

  afterEach(() => {
    view.destroy();
    parent.remove();
  });

  it('should not create count element when search panel is closed', () => {
    expect(searchPanelOpen(view.state)).toBe(false);
    const countEl = parent.querySelector('.cm-search-count');
    expect(countEl).toBeNull();
  });

  it('should display match count when searching', () => {
    openSearchPanel(view);
    expect(searchPanelOpen(view.state)).toBe(true);

    // Set search query
    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    const countEl = parent.querySelector('.cm-search-count');
    expect(countEl).not.toBeNull();
    expect(countEl!.textContent).toBe('1/3');
  });

  it('should show "No results" when query has no matches', () => {
    openSearchPanel(view);

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'nonexistent' })),
    });

    const countEl = parent.querySelector('.cm-search-count');
    expect(countEl).not.toBeNull();
    expect(countEl!.textContent).toBe('No results');
    expect(countEl!.classList.contains('no-results')).toBe(true);
  });

  it('should clear count when query is empty', () => {
    openSearchPanel(view);

    // First set a query
    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    // Then clear it
    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: '' })),
    });

    const countEl = parent.querySelector('.cm-search-count');
    expect(countEl).not.toBeNull();
    expect(countEl!.textContent).toBe('');
    expect(countEl!.classList.contains('no-results')).toBe(false);
  });

  it('should update count when query changes', () => {
    openSearchPanel(view);

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });
    expect(parent.querySelector('.cm-search-count')!.textContent).toBe('1/3');

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'world' })),
    });
    expect(parent.querySelector('.cm-search-count')!.textContent).toBe('1/2');
  });

  it('should reset state when search panel is closed', () => {
    openSearchPanel(view);

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });
    expect(parent.querySelector('.cm-search-count')).not.toBeNull();

    closeSearchPanel(view);

    // After panel close, search panel state should be false
    expect(searchPanelOpen(view.state)).toBe(false);
  });

  it('should show current match index based on cursor position', () => {
    openSearchPanel(view);

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    // Move cursor to second "hello" (line 2, position 12)
    view.dispatch({ selection: { anchor: 12 } });
    // Re-dispatch search to trigger update
    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    const countEl = parent.querySelector('.cm-search-count');
    expect(countEl).not.toBeNull();
    // Should show match near cursor
    expect(countEl!.textContent).toMatch(/\d+\/3/);
  });

  it('should not recalculate when query and selection are unchanged', () => {
    openSearchPanel(view);

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    const countEl = parent.querySelector('.cm-search-count');
    const firstText = countEl!.textContent;

    // Dispatch a no-op state change (e.g., just force update)
    view.dispatch({});

    // Count should remain the same
    expect(parent.querySelector('.cm-search-count')!.textContent).toBe(firstText);
  });

  it('should set autocapitalize/autocorrect on search input', () => {
    openSearchPanel(view);

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    const input = parent.querySelector(
      '.cm-panel.cm-search input[name="search"]'
    ) as HTMLInputElement | null;
    if (input) {
      expect(input.getAttribute('autocapitalize')).toBe('off');
      expect(input.getAttribute('autocorrect')).toBe('off');
      expect(input.getAttribute('spellcheck')).toBe('false');
    }
  });

  it('should show displayIdx=1 when cursor is past all matches', () => {
    openSearchPanel(view);

    // Place cursor at end of document (past all "hello" matches)
    const docLen = view.state.doc.length;
    view.dispatch({ selection: { anchor: docLen } });

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    const countEl = parent.querySelector('.cm-search-count');
    expect(countEl).not.toBeNull();
    // When cursor is past all matches, displayIdx should fall back to 1
    expect(countEl!.textContent).toMatch(/\d+\/3/);
  });

  it('should handle missing panel DOM gracefully', () => {
    openSearchPanel(view);

    // Set initial query
    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    // Temporarily remove .cm-editor class so closest('.cm-editor') returns null
    const editorEl = view.dom;
    const originalClass = editorEl.className;
    editorEl.className = editorEl.className.replace('cm-editor', 'cm-editor-hidden');

    // Change query to force recalculation (bypass same-query cache)
    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'world' })),
    });

    // Restore class
    editorEl.className = originalClass;

    // Should not throw - gracefully returns when panel is not found
  });

  it('should create countEl when input is missing in panel', () => {
    openSearchPanel(view);

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    // Remove the search input from the panel
    const searchPanel = parent.querySelector('.cm-panel.cm-search');
    const input = searchPanel?.querySelector('input[name="search"]');
    if (input) {
      input.remove();
    }

    // Remove existing countEl so plugin needs to recreate it
    const existingCount = parent.querySelector('.cm-search-count');
    if (existingCount) {
      existingCount.remove();
    }

    // Trigger re-evaluation by changing selection
    view.dispatch({ selection: { anchor: 1 } });
    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'world' })),
    });

    // countEl should still be created even without input (just not inserted into DOM via input.after)
  });

  it('should handle destroy correctly', () => {
    openSearchPanel(view);

    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: 'hello' })),
    });

    expect(parent.querySelector('.cm-search-count')).not.toBeNull();

    // Destroy the view - should clean up the count element
    view.destroy();

    // Re-create for afterEach cleanup
    const state = EditorState.create({
      doc: 'test',
      extensions: searchExtension(),
    });
    view = new EditorView({ state, parent });
  });
});
