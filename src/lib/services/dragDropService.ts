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
};
