import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { toggleRemoteAccess, type ToggleRemoteOptions } from './remoteAccessToggle';
import { remoteAccessStore, isRemoteActive } from '@/lib/stores/remoteAccessStore';

// Mock dependencies
vi.mock('@/lib/services/persistenceService', () => ({
  loadRemoteAccessSettings: vi.fn(),
  saveRemoteAccessSettings: vi.fn(),
}));

vi.mock('@/lib/services/remoteAccessService', () => ({
  remoteAccessService: {
    startServer: vi.fn(),
    stopServer: vi.fn(),
    startTunnel: vi.fn(),
    stopTunnel: vi.fn(),
    isCloudflaredAvailable: vi.fn(),
  },
}));

import {
  loadRemoteAccessSettings,
  saveRemoteAccessSettings,
} from '@/lib/services/persistenceService';
import { remoteAccessService } from '@/lib/services/remoteAccessService';

const mockLoad = vi.mocked(loadRemoteAccessSettings);
const mockSave = vi.mocked(saveRemoteAccessSettings);
const mockService = vi.mocked(remoteAccessService);

function createOpts(): ToggleRemoteOptions & { toggleCalls: boolean[]; errorCalls: string[] } {
  const toggleCalls: boolean[] = [];
  const errorCalls: string[] = [];
  return {
    onToggling: (v: boolean) => toggleCalls.push(v),
    onError: (msg: string) => errorCalls.push(msg),
    toggleCalls,
    errorCalls,
  };
}

describe('toggleRemoteAccess', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    remoteAccessStore.reset();
  });

  describe('turning ON', () => {
    beforeEach(() => {
      mockLoad.mockResolvedValue({ port: 9876, tunnelToken: null, enabled: false });
      mockSave.mockResolvedValue(undefined);
      mockService.isCloudflaredAvailable.mockResolvedValue(true);
      mockService.startServer.mockResolvedValue('test-auth-token-uuid');
      mockService.startTunnel.mockResolvedValue('https://test.trycloudflare.com');
    });

    it('should start server and tunnel successfully', async () => {
      const opts = createOpts();
      const result = await toggleRemoteAccess(opts);

      expect(result).toEqual({ tunnelUrl: 'https://test.trycloudflare.com' });
      expect(mockService.startServer).toHaveBeenCalledWith(9876);
      expect(mockService.startTunnel).toHaveBeenCalledWith(null, 9876);
      expect(get(isRemoteActive)).toBe(true);
      expect(opts.toggleCalls).toEqual([true, false]);
    });

    it('should fail entirely when tunnel fails (no LAN fallback)', async () => {
      mockService.startTunnel.mockRejectedValue(new Error('tunnel error'));
      mockService.stopServer.mockResolvedValue(undefined);

      const opts = createOpts();
      const result = await toggleRemoteAccess(opts);

      expect(result).toBeNull();
      expect(get(isRemoteActive)).toBe(false); // server stopped on tunnel failure
      expect(mockService.stopServer).toHaveBeenCalled(); // cleanup
      expect(opts.errorCalls).toContain('Failed to start Cloudflare Tunnel');
    });

    it('should fail when cloudflared is not available (no LAN fallback)', async () => {
      mockService.isCloudflaredAvailable.mockResolvedValue(false);

      const opts = createOpts();
      const result = await toggleRemoteAccess(opts);

      expect(result).toBeNull();
      expect(mockService.startServer).not.toHaveBeenCalled();
      expect(mockService.startTunnel).not.toHaveBeenCalled();
      expect(get(isRemoteActive)).toBe(false);
      expect(opts.errorCalls).toContain(
        'cloudflared is not installed. Run: brew install cloudflared'
      );
    });

    it('should pass tunnel token from settings', async () => {
      mockLoad.mockResolvedValue({ port: 9876, tunnelToken: 'my-token', enabled: false });

      const opts = createOpts();
      await toggleRemoteAccess(opts);

      expect(mockService.startTunnel).toHaveBeenCalledWith('my-token', 9876);
    });

    it('should pass null token when tunnelToken is empty string', async () => {
      mockLoad.mockResolvedValue({ port: 9876, tunnelToken: '  ', enabled: false });

      const opts = createOpts();
      await toggleRemoteAccess(opts);

      expect(mockService.startTunnel).toHaveBeenCalledWith(null, 9876);
    });

    it('should store auth token from server', async () => {
      const opts = createOpts();
      await toggleRemoteAccess(opts);

      const state = get(remoteAccessStore);
      expect(state.authToken).toBe('test-auth-token-uuid');
      expect(state.hasToken).toBe(true);
    });

    it('should save settings with enabled=true', async () => {
      const opts = createOpts();
      await toggleRemoteAccess(opts);

      expect(mockSave).toHaveBeenCalledWith(expect.objectContaining({ enabled: true }));
    });
  });

  describe('turning OFF', () => {
    beforeEach(() => {
      // Set server as running
      remoteAccessStore.setServerRunning(true);
      remoteAccessStore.setTunnelRunning(true, 'https://test.trycloudflare.com');
      mockLoad.mockResolvedValue({ port: 9876, tunnelToken: null, enabled: true });
      mockSave.mockResolvedValue(undefined);
      mockService.stopTunnel.mockResolvedValue(undefined);
      mockService.stopServer.mockResolvedValue(undefined);
    });

    it('should stop tunnel and server', async () => {
      const opts = createOpts();
      const result = await toggleRemoteAccess(opts);

      expect(result).toBeNull();
      expect(mockService.stopTunnel).toHaveBeenCalled();
      expect(mockService.stopServer).toHaveBeenCalled();
      expect(get(isRemoteActive)).toBe(false);
    });

    it('should clear auth token when stopping', async () => {
      remoteAccessStore.setAuthToken('old-token');

      const opts = createOpts();
      await toggleRemoteAccess(opts);

      const state = get(remoteAccessStore);
      expect(state.authToken).toBeNull();
      expect(state.hasToken).toBe(false);
    });

    it('should save settings with enabled=false', async () => {
      const opts = createOpts();
      await toggleRemoteAccess(opts);

      expect(mockSave).toHaveBeenCalledWith(expect.objectContaining({ enabled: false }));
    });

    it('should handle tunnel stop failure gracefully', async () => {
      mockService.stopTunnel.mockRejectedValue(new Error('not running'));

      const opts = createOpts();
      const result = await toggleRemoteAccess(opts);

      expect(result).toBeNull();
      expect(mockService.stopServer).toHaveBeenCalled(); // server still stopped
    });
  });

  describe('error handling', () => {
    it('should return null and set toggling to false on unexpected error', async () => {
      mockLoad.mockRejectedValue(new Error('load failed'));

      const opts = createOpts();
      const result = await toggleRemoteAccess(opts);

      expect(result).toBeNull();
      expect(opts.toggleCalls).toEqual([true, false]);
    });
  });
});
