import { invoke } from '@tauri-apps/api/core';

export interface CopyResult {
  success: boolean;
  copied: string[];
  errors: CopyError[];
}

export interface CopyError {
  path: string;
  error: string;
}

/**
 * Drag and drop service
 * Wraps Tauri commands for file copy operations
 */
export const dragDropService = {
  /**
   * Copy files/directories to a target directory
   */
  copyToDirectory: (sourcePaths: string[], targetDir: string): Promise<CopyResult> =>
    invoke('copy_paths_to_directory', {
      sourcePaths,
      targetDir,
    }),

  /**
   * Move a file/directory to a target directory
   * @returns Final path of the moved item
   */
  moveToDirectory: (sourcePath: string, targetDir: string): Promise<string> =>
    invoke('move_path', {
      source: sourcePath,
      targetDir,
    }),
};
