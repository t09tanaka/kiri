import { invoke } from '@tauri-apps/api/core';
import type { FileEntry } from '@/lib/components/filetree/types';

/**
 * File system operations service
 * Wraps Tauri file system commands for testability
 */
export const fileService = {
  /**
   * Read file contents
   */
  readFile: (path: string): Promise<string> => invoke('read_file', { path }),

  /**
   * Read directory entries
   */
  readDirectory: (path: string): Promise<FileEntry[]> => invoke('read_directory', { path }),

  /**
   * Get home directory path
   */
  getHomeDirectory: (): Promise<string> => invoke('get_home_directory'),

  /**
   * Reveal file or directory in Finder
   */
  revealInFinder: (path: string): Promise<void> => invoke('reveal_in_finder', { path }),

  /**
   * Delete file or directory
   */
  deletePath: (path: string): Promise<void> => invoke('delete_path', { path }),
};
