import { invoke } from '@tauri-apps/api/core';

/**
 * Remote access service
 * Wraps Tauri remote access commands for testability
 */
export const remoteAccessService = {
  /**
   * Start the remote access HTTP server on the specified port
   */
  startServer: (port: number): Promise<string> => invoke('start_remote_server', { port }),

  /**
   * Stop the remote access HTTP server
   */
  stopServer: (): Promise<void> => invoke('stop_remote_server'),

  /**
   * Check if the remote access server is currently running
   */
  isRunning: (): Promise<boolean> => invoke('is_remote_server_running'),

  /**
   * Generate a QR code encoding the full access URL with token in path.
   * If tunnelUrl is provided, it is used as the base URL.
   */
  generateQrCode: (port: number, tunnelUrl?: string): Promise<string> =>
    invoke('generate_remote_qr_code', { port, tunnelUrl: tunnelUrl ?? null }),

  /**
   * Regenerate the remote access authentication token
   */
  regenerateToken: (): Promise<string> => invoke('regenerate_remote_token'),

  /**
   * Start a Cloudflare tunnel.
   * token: null = Quick Tunnel, string = Named Tunnel.
   * Returns the tunnel URL for Quick Tunnel mode.
   */
  startTunnel: (token: string | null, port: number): Promise<string | null> =>
    invoke('start_cloudflare_tunnel', { token, port }),

  /**
   * Stop the Cloudflare tunnel
   */
  stopTunnel: (): Promise<void> => invoke('stop_cloudflare_tunnel'),

  /**
   * Check if cloudflared binary is available on the system
   */
  isCloudflaredAvailable: (): Promise<boolean> => invoke('is_cloudflared_available'),
};
