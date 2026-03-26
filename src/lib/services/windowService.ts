import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

/**
 * Window management service
 * Wraps Tauri window commands for testability
 */
export const windowService = {
  /**
   * Create a new window, optionally opening a project path
   */
  createWindow: (options?: { projectPath?: string | null }): Promise<void> =>
    invoke('create_window', {
      projectPath: options?.projectPath ?? null,
    }),

  /**
   * Focus an existing window for the project path, or create a new one if not found.
   * Returns true if an existing window was focused, false if a new window was created.
   */
  focusOrCreateWindow: (projectPath: string): Promise<boolean> =>
    invoke('focus_or_create_window', { projectPath }),

  /**
   * Register a window with a project path (for windows not created via createWindow)
   */
  registerWindow: (label: string, projectPath: string, isWorktree?: boolean): Promise<void> =>
    invoke('register_window', { label, projectPath, isWorktree: isWorktree ?? false }),

  /**
   * Unregister the current window from the registry (call on window close)
   */
  unregisterWindow: (label: string): Promise<void> => invoke('unregister_window', { label }),

  /**
   * Set the window title
   */
  setTitle: async (title: string): Promise<void> => {
    const window = getCurrentWindow();
    await window.setTitle(title);
  },

  /**
   * Set the size and center the window on screen
   */
  setSizeAndCenter: async (width: number, height: number): Promise<void> => {
    const window = getCurrentWindow();
    await window.setSize(new LogicalSize(width, height));
    await window.center();
  },
};
