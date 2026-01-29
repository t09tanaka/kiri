import { invoke } from '@tauri-apps/api/core';

/**
 * Window management service
 * Wraps Tauri window commands for testability
 */
export const windowService = {
  /**
   * Create a new window, optionally opening a project path
   */
  createWindow: (options?: {
    x?: number | null;
    y?: number | null;
    width?: number | null;
    height?: number | null;
    projectPath?: string | null;
  }): Promise<void> =>
    invoke('create_window', {
      x: options?.x ?? null,
      y: options?.y ?? null,
      width: options?.width ?? null,
      height: options?.height ?? null,
      projectPath: options?.projectPath ?? null,
    }),
};
