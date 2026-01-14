import { invoke } from '@tauri-apps/api/core';

/**
 * Window management service
 * Wraps Tauri window commands for testability
 */
export const windowService = {
  /**
   * Create a new DiffView window
   * @param projectPath - The project path to display diffs for
   */
  createDiffViewWindow: (projectPath: string): Promise<string> =>
    invoke('create_diffview_window', { projectPath }),
};
