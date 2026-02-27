import { invoke } from '@tauri-apps/api/core';

/**
 * Remote access service
 * Wraps Tauri remote access commands for testability
 */
export const remoteAccessService = {
  /**
   * Start the remote access HTTP server on the specified port
   */
  startServer: (port: number): Promise<void> => invoke('start_remote_server', { port }),

  /**
   * Stop the remote access HTTP server
   */
  stopServer: (): Promise<void> => invoke('stop_remote_server'),

  /**
   * Check if the remote access server is currently running
   */
  isRunning: (): Promise<boolean> => invoke('is_remote_server_running'),

  /**
   * Generate a QR code for remote access authentication
   */
  generateQrCode: (): Promise<string> => invoke('generate_remote_qr_code'),

  /**
   * Regenerate the remote access authentication token
   */
  regenerateToken: (): Promise<string> => invoke('regenerate_remote_token'),

  /**
   * Start a Cloudflare tunnel for external access
   */
  startTunnel: (token: string): Promise<void> => invoke('start_cloudflare_tunnel', { token }),

  /**
   * Stop the Cloudflare tunnel
   */
  stopTunnel: (): Promise<void> => invoke('stop_cloudflare_tunnel'),
};
