import { open, type OpenDialogOptions } from '@tauri-apps/plugin-dialog';

/**
 * Native dialog service
 * Wraps Tauri dialog plugin for testability
 */
export const dialogService = {
  /**
   * Open a directory picker dialog
   */
  openDirectory: async (options?: Partial<OpenDialogOptions>): Promise<string | null> => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Open Directory',
      ...options,
    });

    if (selected && typeof selected === 'string') {
      return selected;
    }
    return null;
  },

  /**
   * Open a file picker dialog
   */
  openFile: async (options?: Partial<OpenDialogOptions>): Promise<string | null> => {
    const selected = await open({
      directory: false,
      multiple: false,
      ...options,
    });

    if (selected && typeof selected === 'string') {
      return selected;
    }
    return null;
  },
};
