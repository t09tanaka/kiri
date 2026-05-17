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
   * Move file or directory to target directory
   * @returns Final path of the moved item
   */
  movePath: (source: string, targetDir: string): Promise<string> =>
    invoke('move_path', { source, targetDir }),

  /**
   * Create directory (supports nested paths like "test/opt")
   * @param parentPath - Parent directory path
   * @param name - New directory name (can include slashes for nested creation)
   * @returns Full path of the created directory
   */
  createDirectory: (parentPath: string, name: string): Promise<string> =>
    invoke('create_directory', { parentPath, name }),

  /**
   * Create an empty file inside a directory.
   * Rejects names containing path separators. Errors if the target exists.
   */
  createFile: (parentPath: string, name: string): Promise<string> =>
    invoke('create_file', { parentPath, name }),

  /**
   * Rename a file or directory in place (same parent).
   * Rejects names containing path separators or `.` / `..`.
   */
  renamePath: (path: string, newName: string): Promise<string> =>
    invoke('rename_path', { path, newName }),

  /**
   * Move a file or directory to the OS trash / recycle bin.
   * Reversible within a session via {@link restoreFromTrash} on platforms
   * where {@link trashRestoreSupported} returns true.
   */
  moveToTrash: (path: string): Promise<void> => invoke('move_to_trash', { path }),

  /**
   * Restore the most recent trash entry whose original location matches
   * `originalPath`. Returns the restored path.
   * Errors on macOS / iOS / Android — see {@link trashRestoreSupported}.
   */
  restoreFromTrash: (originalPath: string): Promise<string> =>
    invoke('restore_from_trash', { originalPath }),

  /**
   * Whether the current OS supports programmatic restoration from trash.
   */
  trashRestoreSupported: (): Promise<boolean> => invoke('trash_restore_supported'),

  /**
   * Open the OS-native terminal app at `path` (or its parent if `path` is a file).
   */
  openTerminalHere: (path: string): Promise<void> => invoke('open_terminal_here', { path }),
};
