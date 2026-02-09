import { invoke } from '@tauri-apps/api/core';
import type { FileEntry } from '@/lib/components/filetree/types';

/**
 * File system operations service
 * Wraps Tauri file system commands for testability
 */
export const fileService = {
  /**
   * Read file contents as UTF-8 text
   */
  readFile: (path: string): Promise<string> => invoke('read_file', { path }),

  /**
   * Read file contents as base64-encoded string (for binary files like images)
   */
  readFileAsBase64: (path: string): Promise<string> => invoke('read_file_as_base64', { path }),

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

  /**
   * Create directory (supports nested paths like "test/opt")
   * @param parentPath - Parent directory path
   * @param name - New directory name (can include slashes for nested creation)
   * @returns Full path of the created directory
   */
  createDirectory: (parentPath: string, name: string): Promise<string> =>
    invoke('create_directory', { parentPath, name }),
};
