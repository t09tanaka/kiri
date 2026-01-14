import { invoke } from '@tauri-apps/api/core';

/**
 * Terminal/PTY operations service
 * Wraps Tauri terminal commands for testability
 */
export const terminalService = {
  /**
   * Create a new terminal/PTY instance
   */
  createTerminal: (cwd: string | null, cols: number, rows: number): Promise<number> =>
    invoke('create_terminal', { cwd, cols, rows }),

  /**
   * Write data to terminal
   */
  writeTerminal: (id: number, data: string): Promise<void> =>
    invoke('write_terminal', { id, data }),

  /**
   * Resize terminal
   */
  resizeTerminal: (id: number, cols: number, rows: number): Promise<void> =>
    invoke('resize_terminal', { id, cols, rows }),

  /**
   * Close terminal
   */
  closeTerminal: (id: number): Promise<void> => invoke('close_terminal', { id }),
};
