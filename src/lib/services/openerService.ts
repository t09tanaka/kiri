import { openUrl } from '@tauri-apps/plugin-opener';

/**
 * Opener service for opening URLs and files in external applications
 * Wraps Tauri opener plugin for testability
 */
export const openerService = {
  /**
   * Open a URL in the default browser
   * @param url The URL to open
   */
  openUrl: async (url: string): Promise<void> => {
    await openUrl(url);
  },
};
