import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import {
  contentSearchStore,
  isContentSearchOpen,
  contentSearchResults,
  isContentSearching,
  contentSearchQuery,
  isContentSearchSettingsOpen,
} from './contentSearchStore';

// Mock the search service
vi.mock('@/lib/services/searchService', () => ({
  searchService: {
    searchContent: vi.fn().mockResolvedValue([]),
  },
}));

// Mock the persistence service
vi.mock('@/lib/services/persistenceService', () => ({
  loadProjectSettings: vi.fn().mockResolvedValue({
    searchExcludePatterns: ['*.min.js'],
  }),
  saveProjectSettings: vi.fn().mockResolvedValue(undefined),
  DEFAULT_EXCLUDE_PATTERNS: ['*.min.js', '*.map'],
}));

describe('contentSearchStore', () => {
  beforeEach(() => {
    contentSearchStore.reset();
  });

  describe('initial state', () => {
    it('should start with isOpen as false', () => {
      const state = get(contentSearchStore);
      expect(state.isOpen).toBe(false);
    });

    it('should start with empty query', () => {
      const state = get(contentSearchStore);
      expect(state.query).toBe('');
    });

    it('should start with empty results', () => {
      const state = get(contentSearchStore);
      expect(state.results).toEqual([]);
    });

    it('should start with isSearching as false', () => {
      const state = get(contentSearchStore);
      expect(state.isSearching).toBe(false);
    });

    it('should start with selectedFileIndex as 0', () => {
      const state = get(contentSearchStore);
      expect(state.selectedFileIndex).toBe(0);
    });

    it('should start with selectedMatchIndex as 0', () => {
      const state = get(contentSearchStore);
      expect(state.selectedMatchIndex).toBe(0);
    });

    it('should start with isSettingsOpen as false', () => {
      const state = get(contentSearchStore);
      expect(state.isSettingsOpen).toBe(false);
    });
  });

  describe('open', () => {
    it('should set isOpen to true', async () => {
      await contentSearchStore.open('/path/to/project');
      const state = get(contentSearchStore);
      expect(state.isOpen).toBe(true);
    });

    it('should set projectPath to the provided path', async () => {
      await contentSearchStore.open('/path/to/project');
      const state = get(contentSearchStore);
      expect(state.projectPath).toBe('/path/to/project');
    });

    it('should reset query when opening', async () => {
      contentSearchStore.search('test');
      await contentSearchStore.open('/path/to/project');
      const state = get(contentSearchStore);
      expect(state.query).toBe('');
    });

    it('should reset results when opening', async () => {
      await contentSearchStore.open('/path/to/project');
      const state = get(contentSearchStore);
      expect(state.results).toEqual([]);
    });

    it('should close settings panel when opening', async () => {
      contentSearchStore.toggleSettings();
      await contentSearchStore.open('/path/to/project');
      const state = get(contentSearchStore);
      expect(state.isSettingsOpen).toBe(false);
    });
  });

  describe('close', () => {
    it('should set isOpen to false', async () => {
      await contentSearchStore.open('/path/to/project');
      contentSearchStore.close();
      const state = get(contentSearchStore);
      expect(state.isOpen).toBe(false);
    });

    it('should set isSearching to false', async () => {
      await contentSearchStore.open('/path/to/project');
      contentSearchStore.close();
      const state = get(contentSearchStore);
      expect(state.isSearching).toBe(false);
    });

    it('should close settings panel', async () => {
      await contentSearchStore.open('/path/to/project');
      contentSearchStore.toggleSettings();
      contentSearchStore.close();
      const state = get(contentSearchStore);
      expect(state.isSettingsOpen).toBe(false);
    });
  });

  describe('toggle', () => {
    it('should open if closed', async () => {
      await contentSearchStore.toggle('/path/to/project');
      expect(get(isContentSearchOpen)).toBe(true);
    });

    it('should close if open', async () => {
      await contentSearchStore.open('/path/to/project');
      await contentSearchStore.toggle('/path/to/project');
      expect(get(isContentSearchOpen)).toBe(false);
    });
  });

  describe('search', () => {
    it('should update query', () => {
      contentSearchStore.search('test');
      expect(get(contentSearchQuery)).toBe('test');
    });

    it('should not search with query less than 2 characters', () => {
      contentSearchStore.search('a');
      const state = get(contentSearchStore);
      expect(state.isSearching).toBe(false);
      expect(state.results).toEqual([]);
    });

    it('should clear results with empty query', () => {
      contentSearchStore.search('');
      expect(get(contentSearchResults)).toEqual([]);
    });
  });

  describe('file selection', () => {
    it('should select next file', async () => {
      await contentSearchStore.open('/path/to/project');
      contentSearchStore.selectNextFile();
      // With empty results, should stay at 0 or go to -1 (clamped)
      const state = get(contentSearchStore);
      expect(state.selectedFileIndex).toBe(-1); // min of 0 - 1, results.length - 1 = -1
    });

    it('should select previous file', async () => {
      await contentSearchStore.open('/path/to/project');
      contentSearchStore.selectPreviousFile();
      const state = get(contentSearchStore);
      expect(state.selectedFileIndex).toBe(0); // max of 0, 0 - 1 = 0
    });

    it('should select specific file by index', async () => {
      await contentSearchStore.open('/path/to/project');
      contentSearchStore.selectFile(5);
      const state = get(contentSearchStore);
      // With empty results, Math.min(-1, 5) = -1, Math.max(0, -1) = 0
      expect(state.selectedFileIndex).toBe(0);
    });

    it('should reset selectedMatchIndex when selecting a file', async () => {
      await contentSearchStore.open('/path/to/project');
      contentSearchStore.selectFile(0);
      const state = get(contentSearchStore);
      expect(state.selectedMatchIndex).toBe(0);
    });
  });

  describe('settings panel', () => {
    it('should toggle settings panel', async () => {
      await contentSearchStore.open('/path/to/project');
      expect(get(isContentSearchSettingsOpen)).toBe(false);

      contentSearchStore.toggleSettings();
      expect(get(isContentSearchSettingsOpen)).toBe(true);

      contentSearchStore.toggleSettings();
      expect(get(isContentSearchSettingsOpen)).toBe(false);
    });

    it('should close settings panel', async () => {
      await contentSearchStore.open('/path/to/project');
      contentSearchStore.toggleSettings();
      contentSearchStore.closeSettings();
      expect(get(isContentSearchSettingsOpen)).toBe(false);
    });
  });

  describe('getSelectedFile', () => {
    it('should return null when no results', () => {
      const file = contentSearchStore.getSelectedFile();
      expect(file).toBeNull();
    });
  });

  describe('getSelectedMatch', () => {
    it('should return null when no results', () => {
      const match = contentSearchStore.getSelectedMatch();
      expect(match).toBeNull();
    });
  });

  describe('reset', () => {
    it('should reset to initial state', async () => {
      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('test');
      contentSearchStore.toggleSettings();

      contentSearchStore.reset();

      const state = get(contentSearchStore);
      expect(state.isOpen).toBe(false);
      expect(state.query).toBe('');
      expect(state.results).toEqual([]);
      expect(state.isSearching).toBe(false);
      expect(state.isSettingsOpen).toBe(false);
    });
  });

  describe('derived stores', () => {
    it('isContentSearchOpen should derive correctly', async () => {
      expect(get(isContentSearchOpen)).toBe(false);
      await contentSearchStore.open('/path/to/project');
      expect(get(isContentSearchOpen)).toBe(true);
      contentSearchStore.close();
      expect(get(isContentSearchOpen)).toBe(false);
    });

    it('isContentSearching should derive correctly', () => {
      expect(get(isContentSearching)).toBe(false);
    });

    it('contentSearchQuery should derive correctly', () => {
      expect(get(contentSearchQuery)).toBe('');
      contentSearchStore.search('hello');
      expect(get(contentSearchQuery)).toBe('hello');
    });

    it('contentSearchResults should derive correctly', () => {
      expect(get(contentSearchResults)).toEqual([]);
    });
  });
});
