import { invoke } from '@tauri-apps/api/core';

/**
 * File system watcher service
 * Wraps Tauri watcher commands for testability
 */
export const watcherService = {
  /**
   * Start watching a path for changes
   */
  startWatching: (path: string): Promise<void> => invoke('start_watching', { path }),

  /**
   * Stop watching a path
   */
  stopWatching: (path: string): Promise<void> => invoke('stop_watching', { path }),
};
