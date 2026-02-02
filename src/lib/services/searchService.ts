import { invoke } from '@tauri-apps/api/core';

/**
 * Result of a content match within a file
 */
export interface ContentMatch {
  line: number;
  content: string;
  start: number;
  end: number;
}

/**
 * Result of searching content within files
 */
export interface ContentSearchResult {
  path: string;
  name: string;
  matches: ContentMatch[];
}

/**
 * Search service for Tauri API wrappers
 */
export const searchService = {
  /**
   * Search file contents across a project
   * @param rootPath Root directory to search in
   * @param query Search query (minimum 2 characters)
   * @param maxResults Maximum number of files to return
   * @param excludePatterns Custom patterns to exclude (in addition to defaults)
   * @returns Array of files with matching content
   */
  async searchContent(
    rootPath: string,
    query: string,
    maxResults: number = 100,
    excludePatterns: string[] = []
  ): Promise<ContentSearchResult[]> {
    return invoke<ContentSearchResult[]>('search_content', {
      rootPath,
      query,
      maxResults,
      excludePatterns,
    });
  },
};
