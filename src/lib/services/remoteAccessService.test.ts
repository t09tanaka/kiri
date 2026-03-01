import { describe, it, expect, vi, beforeEach } from 'vitest';
import { remoteAccessService } from './remoteAccessService';

// Mock the Tauri core invoke function
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

describe('remoteAccessService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('startServer', () => {
    it('should call invoke with correct command and port', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      await remoteAccessService.startServer(9876);

      expect(invoke).toHaveBeenCalledWith('start_remote_server', { port: 9876 });
      expect(invoke).toHaveBeenCalledTimes(1);
    });

    it('should handle custom port', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      await remoteAccessService.startServer(8080);

      expect(invoke).toHaveBeenCalledWith('start_remote_server', { port: 8080 });
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Port already in use'));

      await expect(remoteAccessService.startServer(9876)).rejects.toThrow('Port already in use');
    });
  });

  describe('stopServer', () => {
    it('should call invoke with correct command', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      await remoteAccessService.stopServer();

      expect(invoke).toHaveBeenCalledWith('stop_remote_server');
      expect(invoke).toHaveBeenCalledTimes(1);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Server not running'));

      await expect(remoteAccessService.stopServer()).rejects.toThrow('Server not running');
    });
  });

  describe('isRunning', () => {
    it('should call invoke and return true when server is running', async () => {
      vi.mocked(invoke).mockResolvedValue(true);

      const result = await remoteAccessService.isRunning();

      expect(invoke).toHaveBeenCalledWith('is_remote_server_running');
      expect(result).toBe(true);
    });

    it('should call invoke and return false when server is not running', async () => {
      vi.mocked(invoke).mockResolvedValue(false);

      const result = await remoteAccessService.isRunning();

      expect(invoke).toHaveBeenCalledWith('is_remote_server_running');
      expect(result).toBe(false);
    });
  });

  describe('generateQrCode', () => {
    it('should call invoke with port and no tunnel URL', async () => {
      const mockQrCode = 'data:image/png;base64,iVBOR...';
      vi.mocked(invoke).mockResolvedValue(mockQrCode);

      const result = await remoteAccessService.generateQrCode(9876);

      expect(invoke).toHaveBeenCalledWith('generate_remote_qr_code', {
        port: 9876,
        tunnelUrl: null,
      });
      expect(result).toBe(mockQrCode);
    });

    it('should call invoke with port and tunnel URL', async () => {
      const mockQrCode = 'data:image/png;base64,iVBOR...';
      vi.mocked(invoke).mockResolvedValue(mockQrCode);

      const result = await remoteAccessService.generateQrCode(
        9876,
        'https://my-tunnel.trycloudflare.com'
      );

      expect(invoke).toHaveBeenCalledWith('generate_remote_qr_code', {
        port: 9876,
        tunnelUrl: 'https://my-tunnel.trycloudflare.com',
      });
      expect(result).toBe(mockQrCode);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Server not running'));

      await expect(remoteAccessService.generateQrCode(9876)).rejects.toThrow('Server not running');
    });
  });

  describe('regenerateToken', () => {
    it('should call invoke and return new token', async () => {
      const mockToken = 'new-auth-token-abc123';
      vi.mocked(invoke).mockResolvedValue(mockToken);

      const result = await remoteAccessService.regenerateToken();

      expect(invoke).toHaveBeenCalledWith('regenerate_remote_token');
      expect(result).toBe(mockToken);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Failed to generate token'));

      await expect(remoteAccessService.regenerateToken()).rejects.toThrow(
        'Failed to generate token'
      );
    });
  });

  describe('startTunnel', () => {
    it('should call invoke with named tunnel token and port', async () => {
      vi.mocked(invoke).mockResolvedValue(null);

      const result = await remoteAccessService.startTunnel('my-cloudflare-token', 9876);

      expect(invoke).toHaveBeenCalledWith('start_cloudflare_tunnel', {
        token: 'my-cloudflare-token',
        port: 9876,
      });
      expect(invoke).toHaveBeenCalledTimes(1);
      expect(result).toBeNull();
    });

    it('should call invoke with null token for Quick Tunnel and return URL', async () => {
      const tunnelUrl = 'https://random-words.trycloudflare.com';
      vi.mocked(invoke).mockResolvedValue(tunnelUrl);

      const result = await remoteAccessService.startTunnel(null, 9876);

      expect(invoke).toHaveBeenCalledWith('start_cloudflare_tunnel', {
        token: null,
        port: 9876,
      });
      expect(result).toBe(tunnelUrl);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Tunnel failed to start'));

      await expect(remoteAccessService.startTunnel('token', 9876)).rejects.toThrow(
        'Tunnel failed to start'
      );
    });
  });

  describe('stopTunnel', () => {
    it('should call invoke with correct command', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      await remoteAccessService.stopTunnel();

      expect(invoke).toHaveBeenCalledWith('stop_cloudflare_tunnel');
      expect(invoke).toHaveBeenCalledTimes(1);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Tunnel not running'));

      await expect(remoteAccessService.stopTunnel()).rejects.toThrow('Tunnel not running');
    });
  });

  describe('isCloudflaredAvailable', () => {
    it('should call invoke with is_cloudflared_available and return true', async () => {
      vi.mocked(invoke).mockResolvedValue(true);

      const result = await remoteAccessService.isCloudflaredAvailable();

      expect(invoke).toHaveBeenCalledWith('is_cloudflared_available');
      expect(result).toBe(true);
    });

    it('should call invoke with is_cloudflared_available and return false', async () => {
      vi.mocked(invoke).mockResolvedValue(false);

      const result = await remoteAccessService.isCloudflaredAvailable();

      expect(invoke).toHaveBeenCalledWith('is_cloudflared_available');
      expect(result).toBe(false);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Command failed'));

      await expect(remoteAccessService.isCloudflaredAvailable()).rejects.toThrow('Command failed');
    });
  });
});
