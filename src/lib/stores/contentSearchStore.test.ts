import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { get } from 'svelte/store';
import {
  contentSearchStore,
  isContentSearchOpen,
  contentSearchResults,
  isContentSearching,
  contentSearchQuery,
  isContentSearchSettingsOpen,
  selectedContentFile,
} from './contentSearchStore';
import { searchService } from '@/lib/services/searchService';
import { saveProjectSettings } from '@/lib/services/persistenceService';

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

const mockSearchResults = [
  {
    path: '/project/src/foo.ts',
    name: 'foo.ts',
    matches: [
      { line: 10, content: 'function foo() {', start: 9, end: 12 },
      { line: 20, content: 'const foo = bar;', start: 6, end: 9 },
    ],
  },
  {
    path: '/project/src/bar.ts',
    name: 'bar.ts',
    matches: [{ line: 5, content: 'import { foo } from "./foo";', start: 10, end: 13 }],
  },
];

describe('contentSearchStore', () => {
  beforeEach(() => {
    contentSearchStore.reset();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
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

  describe('search with debounced execution', () => {
    it('should call searchService.searchContent after debounce timeout', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue(mockSearchResults);

      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('foo');

      // Before timeout, should be searching
      expect(get(contentSearchStore).isSearching).toBe(true);

      // Advance past the 200ms debounce
      await vi.advanceTimersByTimeAsync(200);

      const state = get(contentSearchStore);
      expect(state.isSearching).toBe(false);
      expect(state.results).toEqual(mockSearchResults);
      expect(state.selectedFileIndex).toBe(0);
      expect(state.selectedMatchIndex).toBe(0);
      expect(state.error).toBeNull();
      expect(mockSearchContent).toHaveBeenCalledWith('/path/to/project', 'foo', 100, ['*.min.js']);
    });

    it('should set error when searchService.searchContent throws an Error', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockRejectedValue(new Error('Search failed unexpectedly'));

      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('foo');

      await vi.advanceTimersByTimeAsync(200);

      const state = get(contentSearchStore);
      expect(state.isSearching).toBe(false);
      expect(state.error).toBe('Search failed unexpectedly');
      expect(state.results).toEqual([]);
    });

    it('should set generic error when searchService throws a non-Error', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockRejectedValue('string error');

      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('foo');

      await vi.advanceTimersByTimeAsync(200);

      const state = get(contentSearchStore);
      expect(state.isSearching).toBe(false);
      expect(state.error).toBe('Search failed');
    });

    it('should set error when projectPath is null', async () => {
      vi.useFakeTimers();

      // Search without opening (projectPath is null)
      contentSearchStore.search('foo');

      await vi.advanceTimersByTimeAsync(200);

      const state = get(contentSearchStore);
      expect(state.isSearching).toBe(false);
      expect(state.error).toBe('No project path');
    });

    it('should debounce multiple rapid search calls', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue(mockSearchResults);

      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('fo');
      contentSearchStore.search('foo');
      contentSearchStore.search('foob');

      await vi.advanceTimersByTimeAsync(200);

      // Only the last query should be used
      expect(mockSearchContent).toHaveBeenCalledTimes(1);
      expect(mockSearchContent).toHaveBeenCalledWith('/path/to/project', 'foob', 100, ['*.min.js']);
    });

    it('should cancel pending search timeout on close', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue(mockSearchResults);

      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('foo');

      // Close before the timeout fires
      contentSearchStore.close();

      await vi.advanceTimersByTimeAsync(200);

      // searchContent should not have been called since close clears the timeout
      expect(mockSearchContent).not.toHaveBeenCalled();
    });
  });

  describe('open with same project path', () => {
    it('should not reload settings when opening the same project', async () => {
      // First open loads settings
      await contentSearchStore.open('/path/to/project');
      const state1 = get(contentSearchStore);
      expect(state1.excludePatterns).toEqual(['*.min.js']);

      // Close and reopen the same project
      contentSearchStore.close();
      await contentSearchStore.open('/path/to/project');
      const state2 = get(contentSearchStore);
      expect(state2.isOpen).toBe(true);
      // Patterns should be preserved from the original state (not reloaded)
      expect(state2.excludePatterns).toEqual(['*.min.js']);
    });
  });

  describe('match selection with no results', () => {
    it('selectPreviousMatch should return state unchanged when no results', () => {
      contentSearchStore.selectPreviousMatch();
      const state = get(contentSearchStore);
      expect(state.selectedMatchIndex).toBe(0);
    });

    it('selectNextMatch should return state unchanged when no results', () => {
      contentSearchStore.selectNextMatch();
      const state = get(contentSearchStore);
      expect(state.selectedMatchIndex).toBe(0);
    });
  });

  describe('match selection with results', () => {
    beforeEach(async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue(mockSearchResults);

      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('foo');
      await vi.advanceTimersByTimeAsync(200);
    });

    it('should select next match within current file', () => {
      contentSearchStore.selectNextMatch();
      const state = get(contentSearchStore);
      expect(state.selectedMatchIndex).toBe(1);
    });

    it('should not exceed max match index', () => {
      contentSearchStore.selectNextMatch();
      contentSearchStore.selectNextMatch();
      const state = get(contentSearchStore);
      // foo.ts has 2 matches, max index is 1
      expect(state.selectedMatchIndex).toBe(1);
    });

    it('should select previous match', () => {
      contentSearchStore.selectNextMatch();
      contentSearchStore.selectPreviousMatch();
      const state = get(contentSearchStore);
      expect(state.selectedMatchIndex).toBe(0);
    });

    it('should not go below 0 for match index', () => {
      contentSearchStore.selectPreviousMatch();
      const state = get(contentSearchStore);
      expect(state.selectedMatchIndex).toBe(0);
    });

    it('should select next file and reset match index', () => {
      contentSearchStore.selectNextMatch(); // matchIndex = 1
      contentSearchStore.selectNextFile(); // fileIndex = 1, matchIndex = 0
      const state = get(contentSearchStore);
      expect(state.selectedFileIndex).toBe(1);
      expect(state.selectedMatchIndex).toBe(0);
    });

    it('should not exceed max file index', () => {
      contentSearchStore.selectNextFile();
      contentSearchStore.selectNextFile();
      const state = get(contentSearchStore);
      // 2 files, max index is 1
      expect(state.selectedFileIndex).toBe(1);
    });

    it('should select previous file', () => {
      contentSearchStore.selectNextFile();
      contentSearchStore.selectPreviousFile();
      const state = get(contentSearchStore);
      expect(state.selectedFileIndex).toBe(0);
    });

    it('should clamp selectFile to valid range', () => {
      contentSearchStore.selectFile(100);
      const state = get(contentSearchStore);
      expect(state.selectedFileIndex).toBe(1); // clamped to results.length - 1
    });

    it('should clamp selectFile to 0 for negative index', () => {
      contentSearchStore.selectFile(-5);
      const state = get(contentSearchStore);
      expect(state.selectedFileIndex).toBe(0);
    });
  });

  describe('getSelectedFile with results', () => {
    beforeEach(async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue(mockSearchResults);

      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('foo');
      await vi.advanceTimersByTimeAsync(200);
    });

    it('should return the currently selected file', () => {
      const file = contentSearchStore.getSelectedFile();
      expect(file).toEqual(mockSearchResults[0]);
    });

    it('should return correct file after selecting a different one', () => {
      contentSearchStore.selectNextFile();
      const file = contentSearchStore.getSelectedFile();
      expect(file).toEqual(mockSearchResults[1]);
    });
  });

  describe('getSelectedMatch with results', () => {
    beforeEach(async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue(mockSearchResults);

      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('foo');
      await vi.advanceTimersByTimeAsync(200);
    });

    it('should return the currently selected match', () => {
      const match = contentSearchStore.getSelectedMatch();
      expect(match).toEqual(mockSearchResults[0].matches[0]);
    });

    it('should return correct match after selecting next match', () => {
      contentSearchStore.selectNextMatch();
      const match = contentSearchStore.getSelectedMatch();
      expect(match).toEqual(mockSearchResults[0].matches[1]);
    });

    it('should return correct match from a different file', () => {
      contentSearchStore.selectNextFile();
      const match = contentSearchStore.getSelectedMatch();
      expect(match).toEqual(mockSearchResults[1].matches[0]);
    });
  });

  describe('addExcludePattern', () => {
    beforeEach(async () => {
      await contentSearchStore.open('/path/to/project');
    });

    it('should add a new exclude pattern', async () => {
      await contentSearchStore.addExcludePattern('*.log');
      const state = get(contentSearchStore);
      expect(state.excludePatterns).toContain('*.log');
    });

    it('should trim the pattern before adding', async () => {
      await contentSearchStore.addExcludePattern('  *.log  ');
      const state = get(contentSearchStore);
      expect(state.excludePatterns).toContain('*.log');
    });

    it('should not add empty or whitespace-only patterns', async () => {
      const stateBefore = get(contentSearchStore);
      const countBefore = stateBefore.excludePatterns.length;

      await contentSearchStore.addExcludePattern('');
      await contentSearchStore.addExcludePattern('   ');

      const stateAfter = get(contentSearchStore);
      expect(stateAfter.excludePatterns.length).toBe(countBefore);
    });

    it('should not add duplicate patterns', async () => {
      const stateBefore = get(contentSearchStore);
      const countBefore = stateBefore.excludePatterns.length;

      // '*.min.js' is already in the default patterns loaded from settings
      await contentSearchStore.addExcludePattern('*.min.js');

      const stateAfter = get(contentSearchStore);
      expect(stateAfter.excludePatterns.length).toBe(countBefore);
    });

    it('should save to project settings after adding', async () => {
      const mockSave = vi.mocked(saveProjectSettings);

      await contentSearchStore.addExcludePattern('*.log');

      expect(mockSave).toHaveBeenCalledWith('/path/to/project', {
        searchExcludePatterns: expect.arrayContaining(['*.log']),
      });
    });

    it('should re-run search if query is >= 2 characters', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue([]);

      contentSearchStore.search('foo');
      await vi.advanceTimersByTimeAsync(200);
      mockSearchContent.mockClear();

      await contentSearchStore.addExcludePattern('*.log');

      // The search method is called, which sets a new timeout
      await vi.advanceTimersByTimeAsync(200);
      expect(mockSearchContent).toHaveBeenCalled();
    });

    it('should not re-run search if query is less than 2 characters', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);

      contentSearchStore.search('a'); // Less than 2 chars
      await vi.advanceTimersByTimeAsync(200);
      mockSearchContent.mockClear();

      await contentSearchStore.addExcludePattern('*.log');

      await vi.advanceTimersByTimeAsync(200);
      expect(mockSearchContent).not.toHaveBeenCalled();
    });
  });

  describe('removeExcludePattern', () => {
    beforeEach(async () => {
      await contentSearchStore.open('/path/to/project');
    });

    it('should remove an existing exclude pattern', async () => {
      await contentSearchStore.removeExcludePattern('*.min.js');
      const state = get(contentSearchStore);
      expect(state.excludePatterns).not.toContain('*.min.js');
    });

    it('should save to project settings after removing', async () => {
      const mockSave = vi.mocked(saveProjectSettings);

      await contentSearchStore.removeExcludePattern('*.min.js');

      expect(mockSave).toHaveBeenCalledWith('/path/to/project', {
        searchExcludePatterns: expect.not.arrayContaining(['*.min.js']),
      });
    });

    it('should re-run search if query is >= 2 characters', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue([]);

      contentSearchStore.search('foo');
      await vi.advanceTimersByTimeAsync(200);
      mockSearchContent.mockClear();

      await contentSearchStore.removeExcludePattern('*.min.js');

      await vi.advanceTimersByTimeAsync(200);
      expect(mockSearchContent).toHaveBeenCalled();
    });

    it('should not re-run search if query is less than 2 characters', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);

      contentSearchStore.search('x');
      await vi.advanceTimersByTimeAsync(200);
      mockSearchContent.mockClear();

      await contentSearchStore.removeExcludePattern('*.min.js');

      await vi.advanceTimersByTimeAsync(200);
      expect(mockSearchContent).not.toHaveBeenCalled();
    });

    it('should handle removing a non-existent pattern gracefully', async () => {
      const stateBefore = get(contentSearchStore);
      const patternsBefore = [...stateBefore.excludePatterns];

      await contentSearchStore.removeExcludePattern('nonexistent-pattern');

      const stateAfter = get(contentSearchStore);
      expect(stateAfter.excludePatterns).toEqual(patternsBefore);
    });
  });

  describe('resetExcludePatterns', () => {
    beforeEach(async () => {
      await contentSearchStore.open('/path/to/project');
    });

    it('should reset patterns to DEFAULT_EXCLUDE_PATTERNS', async () => {
      // Add a custom pattern first
      await contentSearchStore.addExcludePattern('*.custom');
      const stateBefore = get(contentSearchStore);
      expect(stateBefore.excludePatterns).toContain('*.custom');

      await contentSearchStore.resetExcludePatterns();

      const state = get(contentSearchStore);
      expect(state.excludePatterns).toEqual(['*.min.js', '*.map']);
    });

    it('should save to project settings after resetting', async () => {
      const mockSave = vi.mocked(saveProjectSettings);

      await contentSearchStore.resetExcludePatterns();

      expect(mockSave).toHaveBeenCalledWith('/path/to/project', {
        searchExcludePatterns: ['*.min.js', '*.map'],
      });
    });

    it('should re-run search if query is >= 2 characters', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue([]);

      contentSearchStore.search('foo');
      await vi.advanceTimersByTimeAsync(200);
      mockSearchContent.mockClear();

      await contentSearchStore.resetExcludePatterns();

      await vi.advanceTimersByTimeAsync(200);
      expect(mockSearchContent).toHaveBeenCalled();
    });

    it('should not re-run search if query is less than 2 characters', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);

      contentSearchStore.search('z');
      await vi.advanceTimersByTimeAsync(200);
      mockSearchContent.mockClear();

      await contentSearchStore.resetExcludePatterns();

      await vi.advanceTimersByTimeAsync(200);
      expect(mockSearchContent).not.toHaveBeenCalled();
    });
  });

  describe('exclude patterns without projectPath', () => {
    it('addExcludePattern should not save when projectPath is null', async () => {
      const mockSave = vi.mocked(saveProjectSettings);

      // Don't open a project, so projectPath is null
      await contentSearchStore.addExcludePattern('*.log');

      expect(mockSave).not.toHaveBeenCalled();
    });

    it('removeExcludePattern should not save when projectPath is null', async () => {
      const mockSave = vi.mocked(saveProjectSettings);

      await contentSearchStore.removeExcludePattern('*.min.js');

      expect(mockSave).not.toHaveBeenCalled();
    });

    it('resetExcludePatterns should not save when projectPath is null', async () => {
      const mockSave = vi.mocked(saveProjectSettings);

      await contentSearchStore.resetExcludePatterns();

      expect(mockSave).not.toHaveBeenCalled();
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

    it('selectedContentFile should return null when no results', () => {
      expect(get(selectedContentFile)).toBeNull();
    });

    it('selectedContentFile should return the selected file', async () => {
      vi.useFakeTimers();
      const mockSearchContent = vi.mocked(searchService.searchContent);
      mockSearchContent.mockResolvedValue(mockSearchResults);

      await contentSearchStore.open('/path/to/project');
      contentSearchStore.search('foo');
      await vi.advanceTimersByTimeAsync(200);

      expect(get(selectedContentFile)).toEqual(mockSearchResults[0]);

      contentSearchStore.selectNextFile();
      expect(get(selectedContentFile)).toEqual(mockSearchResults[1]);
    });
  });
});
