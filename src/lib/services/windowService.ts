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

  /**
   * Focus an existing window for the project path, or create a new one if not found.
   * Returns true if an existing window was focused, false if a new window was created.
   */
  focusOrCreateWindow: (projectPath: string): Promise<boolean> =>
    invoke('focus_or_create_window', { projectPath }),

  /**
   * Register a window with a project path (for windows not created via createWindow)
   */
  registerWindow: (label: string, projectPath: string): Promise<void> =>
    invoke('register_window', { label, projectPath }),

  /**
   * Unregister the current window from the registry (call on window close)
   */
  unregisterWindow: (label: string): Promise<void> => invoke('unregister_window', { label }),

  /**
   * Set the geometry (position and size) of the current window
   */
  setGeometry: async (options: {
    x?: number;
    y?: number;
    width: number;
    height: number;
  }): Promise<void> => {
    const window = getCurrentWindow();
    const label = window.label;

    // Get current position if not provided
    let x = options.x;
    let y = options.y;
    if (x === undefined || y === undefined) {
      const [currentX, currentY] = await invoke<[number, number, number, number]>(
        'get_window_geometry',
        { label }
      );
      x = x ?? currentX;
      y = y ?? currentY;
    }

    await invoke('set_window_geometry', {
      label,
      x,
      y,
      width: options.width,
      height: options.height,
    });
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
