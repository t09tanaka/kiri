import { describe, it, expect, vi, beforeEach } from 'vitest';
import { searchService, type ContentSearchResult } from './searchService';

// Mock the Tauri core invoke function
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

describe('searchService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('searchContent', () => {
    it('should call invoke with correct parameters', async () => {
      const mockResults: ContentSearchResult[] = [
        {
          path: '/path/to/file.ts',
          name: 'file.ts',
          matches: [{ line: 10, content: 'const hello = "world";', start: 6, end: 11 }],
        },
      ];

      vi.mocked(invoke).mockResolvedValue(mockResults);

      const results = await searchService.searchContent('/project', 'hello', 100, ['*.min.js']);

      expect(invoke).toHaveBeenCalledWith('search_content', {
        rootPath: '/project',
        query: 'hello',
        maxResults: 100,
        excludePatterns: ['*.min.js'],
      });
      expect(results).toEqual(mockResults);
    });

    it('should use default maxResults when not specified', async () => {
      vi.mocked(invoke).mockResolvedValue([]);

      await searchService.searchContent('/project', 'hello');

      expect(invoke).toHaveBeenCalledWith('search_content', {
        rootPath: '/project',
        query: 'hello',
        maxResults: 100,
        excludePatterns: [],
      });
    });

    it('should use default excludePatterns when not specified', async () => {
      vi.mocked(invoke).mockResolvedValue([]);

      await searchService.searchContent('/project', 'hello', 50);

      expect(invoke).toHaveBeenCalledWith('search_content', {
        rootPath: '/project',
        query: 'hello',
        maxResults: 50,
        excludePatterns: [],
      });
    });

    it('should return empty array when invoke returns empty', async () => {
      vi.mocked(invoke).mockResolvedValue([]);

      const results = await searchService.searchContent('/project', 'notfound');

      expect(results).toEqual([]);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Path does not exist'));

      await expect(searchService.searchContent('/nonexistent', 'hello')).rejects.toThrow(
        'Path does not exist'
      );
    });

    it('should handle multiple results with multiple matches', async () => {
      const mockResults: ContentSearchResult[] = [
        {
          path: '/project/src/main.ts',
          name: 'main.ts',
          matches: [
            { line: 1, content: 'import test from "test";', start: 7, end: 11 },
            { line: 5, content: 'test();', start: 0, end: 4 },
          ],
        },
        {
          path: '/project/src/utils.ts',
          name: 'utils.ts',
          matches: [{ line: 10, content: 'export const test = () => {};', start: 13, end: 17 }],
        },
      ];

      vi.mocked(invoke).mockResolvedValue(mockResults);

      const results = await searchService.searchContent('/project', 'test');

      expect(results).toHaveLength(2);
      expect(results[0].matches).toHaveLength(2);
      expect(results[1].matches).toHaveLength(1);
    });
  });
});
