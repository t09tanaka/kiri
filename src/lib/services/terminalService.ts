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
   * Write data to terminal (fire-and-forget for low latency)
   */
  writeTerminal: (id: number, data: string): void => {
    invoke('write_terminal', { id, data }).catch((err) => {
      // Log error but don't block - terminal might be closing
      console.warn('[Terminal] Write failed:', err);
    });
  },

  /**
   * Resize terminal
   */
  resizeTerminal: (id: number, cols: number, rows: number): Promise<void> =>
    invoke('resize_terminal', { id, cols, rows }),

  /**
   * Close terminal
   */
  closeTerminal: (id: number): Promise<void> => invoke('close_terminal', { id }),

  /**
   * Check if terminal's shell process is still running
   * Returns true if the process is alive, false if it has exited
   */
  isTerminalAlive: (id: number): Promise<boolean> => invoke('is_terminal_alive', { id }),

  /**
   * Get the foreground process name for a terminal
   * Returns the running command name (e.g., "vim"), shell name (e.g., "zsh"), or "Terminal"
   */
  getProcessName: (id: number): Promise<string> => invoke('get_foreground_process_name', { id }),

  /**
   * Get process info (name + memory usage) for a terminal
   */
  getProcessInfo: (id: number): Promise<{ name: string; memory_bytes: number }> =>
    invoke('get_terminal_process_info', { id }),

  /**
   * Get the current working directory of a terminal
   */
  getCwd: (id: number): Promise<string | null> => invoke('get_terminal_cwd', { id }),
};
