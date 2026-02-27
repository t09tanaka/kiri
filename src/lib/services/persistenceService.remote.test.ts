import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Store } from '@tauri-apps/plugin-store';

// Reset modules before each test to clear the cached store singleton
// This is necessary because persistenceService.ts caches the Store instance
// at module level, and when running the full test suite, another test file
// may have already populated this singleton.
beforeEach(() => {
  vi.resetModules();
});

async function importModule() {
  return await import('./persistenceService');
}

describe('RemoteAccessSettings', () => {
  let mockStore: {
    get: ReturnType<typeof vi.fn>;
    set: ReturnType<typeof vi.fn>;
    save: ReturnType<typeof vi.fn>;
    reload: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    mockStore = {
      get: vi.fn().mockResolvedValue(null),
      set: vi.fn().mockResolvedValue(undefined),
      save: vi.fn().mockResolvedValue(undefined),
      reload: vi.fn().mockResolvedValue(undefined),
    };
    vi.mocked(Store.load).mockResolvedValue(mockStore as unknown as Store);
  });

  describe('DEFAULT_REMOTE_ACCESS_SETTINGS', () => {
    it('should have correct default values', async () => {
      const { DEFAULT_REMOTE_ACCESS_SETTINGS } = await importModule();

      expect(DEFAULT_REMOTE_ACCESS_SETTINGS).toEqual({
        enabled: false,
        port: 9876,
        authToken: null,
        cloudflare: {
          enabled: false,
          tunnelToken: null,
        },
      });
    });
  });

  describe('loadRemoteAccessSettings', () => {
    it('should return default settings when no settings are stored', async () => {
      const { loadRemoteAccessSettings, DEFAULT_REMOTE_ACCESS_SETTINGS } = await importModule();
      mockStore.get.mockResolvedValue(null);

      const result = await loadRemoteAccessSettings();

      expect(result).toEqual(DEFAULT_REMOTE_ACCESS_SETTINGS);
    });

    it('should reload store before reading (multi-window support)', async () => {
      const { loadRemoteAccessSettings } = await importModule();
      const callOrder: string[] = [];
      mockStore.reload.mockImplementation(() => {
        callOrder.push('reload');
        return Promise.resolve();
      });
      mockStore.get.mockImplementation(() => {
        callOrder.push('get');
        return Promise.resolve(null);
      });

      await loadRemoteAccessSettings();

      expect(callOrder[0]).toBe('reload');
      expect(callOrder[1]).toBe('get');
    });

    it('should read from the "remoteAccess" key', async () => {
      const { loadRemoteAccessSettings } = await importModule();
      mockStore.get.mockResolvedValue(null);

      await loadRemoteAccessSettings();

      expect(mockStore.get).toHaveBeenCalledWith('remoteAccess');
    });

    it('should return stored settings when available', async () => {
      const { loadRemoteAccessSettings } = await importModule();
      const storedSettings = {
        enabled: true,
        port: 8080,
        authToken: 'my-secret-token',
        cloudflare: {
          enabled: true,
          tunnelToken: 'cf-tunnel-token',
        },
      };
      mockStore.get.mockResolvedValue(storedSettings);

      const result = await loadRemoteAccessSettings();

      expect(result).toEqual(storedSettings);
    });

    it('should return default settings when store throws an error', async () => {
      const { loadRemoteAccessSettings, DEFAULT_REMOTE_ACCESS_SETTINGS } = await importModule();
      mockStore.reload.mockRejectedValue(new Error('Store error'));

      const result = await loadRemoteAccessSettings();

      expect(result).toEqual(DEFAULT_REMOTE_ACCESS_SETTINGS);
    });

    it('should fill missing cloudflare sub-fields with defaults for partial cloudflare config', async () => {
      const { loadRemoteAccessSettings } = await importModule();
      const partialCloudflareSettings = {
        enabled: true,
        port: 4000,
        authToken: 'token',
        cloudflare: {
          enabled: true,
          // tunnelToken is missing
        },
      };
      mockStore.get.mockResolvedValue(partialCloudflareSettings);

      const result = await loadRemoteAccessSettings();

      expect(result.cloudflare.enabled).toBe(true);
      expect(result.cloudflare.tunnelToken).toBeNull();
    });

    it('should fill missing fields with defaults for partial stored settings', async () => {
      const { loadRemoteAccessSettings } = await importModule();
      // Simulate a scenario where only some fields are stored (e.g., from an older version)
      const partialSettings = {
        enabled: true,
        port: 3000,
      };
      mockStore.get.mockResolvedValue(partialSettings);

      const result = await loadRemoteAccessSettings();

      expect(result.enabled).toBe(true);
      expect(result.port).toBe(3000);
      expect(result.authToken).toBeNull();
      expect(result.cloudflare).toEqual({
        enabled: false,
        tunnelToken: null,
      });
    });
  });

  describe('saveRemoteAccessSettings', () => {
    it('should save settings with the "remoteAccess" key', async () => {
      const { saveRemoteAccessSettings } = await importModule();
      const settings = {
        enabled: true,
        port: 9876,
        authToken: 'test-token',
        cloudflare: {
          enabled: false,
          tunnelToken: null,
        },
      };

      await saveRemoteAccessSettings(settings);

      expect(mockStore.set).toHaveBeenCalledWith('remoteAccess', settings);
    });

    it('should call save after setting values', async () => {
      const { saveRemoteAccessSettings, DEFAULT_REMOTE_ACCESS_SETTINGS } = await importModule();
      const settings = { ...DEFAULT_REMOTE_ACCESS_SETTINGS };

      await saveRemoteAccessSettings(settings);

      expect(mockStore.set).toHaveBeenCalled();
      expect(mockStore.save).toHaveBeenCalled();
    });

    it('should not throw when store save fails', async () => {
      const { saveRemoteAccessSettings, DEFAULT_REMOTE_ACCESS_SETTINGS } = await importModule();
      mockStore.save.mockRejectedValue(new Error('Save error'));
      const settings = { ...DEFAULT_REMOTE_ACCESS_SETTINGS };

      await expect(saveRemoteAccessSettings(settings)).resolves.not.toThrow();
    });
  });
});
