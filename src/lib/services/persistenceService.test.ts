import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Store } from '@tauri-apps/plugin-store';
import {
  STARTUP_COMMANDS,
  DEFAULT_STARTUP_COMMAND,
  getStartupCommandString,
  getDefaultSettings,
  getDefaultProjectSettings,
  DEFAULT_EXCLUDE_PATTERNS,
  DEFAULT_WORKTREE_COPY_PATTERNS,
  type StartupCommand,
} from './persistenceService';

describe('StartupCommand', () => {
  describe('STARTUP_COMMANDS', () => {
    it('should have three options: none, claude, codex', () => {
      expect(STARTUP_COMMANDS).toHaveLength(3);
      expect(STARTUP_COMMANDS.map((c) => c.id)).toEqual(['none', 'claude', 'codex']);
    });

    it('should have labels for each option', () => {
      expect(STARTUP_COMMANDS[0].label).toBe('None');
      expect(STARTUP_COMMANDS[1].label).toBe('Claude');
      expect(STARTUP_COMMANDS[2].label).toBe('Codex');
    });

    it('should have command strings (empty for none)', () => {
      expect(STARTUP_COMMANDS[0].command).toBe('');
      expect(STARTUP_COMMANDS[1].command).toBe('claude');
      expect(STARTUP_COMMANDS[2].command).toBe('codex');
    });
  });

  describe('DEFAULT_STARTUP_COMMAND', () => {
    it('should be none', () => {
      expect(DEFAULT_STARTUP_COMMAND).toBe('none');
    });
  });

  describe('getStartupCommandString', () => {
    it('should return empty string for none', () => {
      expect(getStartupCommandString('none')).toBe('');
    });

    it('should return claude for claude', () => {
      expect(getStartupCommandString('claude')).toBe('claude');
    });

    it('should return codex for codex', () => {
      expect(getStartupCommandString('codex')).toBe('codex');
    });

    it('should return empty string for unknown value', () => {
      expect(getStartupCommandString('unknown' as StartupCommand)).toBe('');
    });
  });
});

describe('getDefaultSettings', () => {
  it('should return default settings object', () => {
    const settings = getDefaultSettings();

    expect(settings.fontSize).toBe(13);
    expect(settings.startupCommand).toBe('none');
  });

  it('should return a new object each time (no shared references)', () => {
    const settings1 = getDefaultSettings();
    const settings2 = getDefaultSettings();

    expect(settings1).toEqual(settings2);
    expect(settings1).not.toBe(settings2);
  });
});

describe('getDefaultProjectSettings', () => {
  it('should return default project settings object', () => {
    const settings = getDefaultProjectSettings();

    expect(settings.searchExcludePatterns).toEqual(DEFAULT_EXCLUDE_PATTERNS);
    expect(settings.worktreeCopyPatterns).toEqual([]);
    expect(settings.worktreeInitCommands).toEqual([]);
  });

  it('should return a new object each time (no shared references)', () => {
    const settings1 = getDefaultProjectSettings();
    const settings2 = getDefaultProjectSettings();

    expect(settings1).toEqual(settings2);
    expect(settings1).not.toBe(settings2);
  });
});

describe('DEFAULT_WORKTREE_COPY_PATTERNS', () => {
  it('should include .env file patterns', () => {
    expect(DEFAULT_WORKTREE_COPY_PATTERNS).toEqual(['**/.env*']);
  });
});

describe('GlobalSettings (with Store mock)', () => {
  let mockStore: {
    get: ReturnType<typeof vi.fn>;
    set: ReturnType<typeof vi.fn>;
    save: ReturnType<typeof vi.fn>;
    reload: ReturnType<typeof vi.fn>;
  };

  // Reset modules to clear the cached store singleton between tests
  beforeEach(() => {
    vi.resetModules();
    mockStore = {
      get: vi.fn().mockResolvedValue(null),
      set: vi.fn().mockResolvedValue(undefined),
      save: vi.fn().mockResolvedValue(undefined),
      reload: vi.fn().mockResolvedValue(undefined),
    };
    vi.mocked(Store.load).mockResolvedValue(mockStore as unknown as Store);
  });

  async function importModule() {
    return await import('./persistenceService');
  }

  describe('loadSettings', () => {
    it('should return default settings when no settings are stored', async () => {
      const { loadSettings } = await importModule();
      mockStore.get.mockResolvedValue(null);

      const result = await loadSettings();

      expect(result.fontSize).toBe(13);
      expect(result.startupCommand).toBe('none');
    });

    it('should reload store before reading', async () => {
      const { loadSettings } = await importModule();
      const callOrder: string[] = [];
      mockStore.reload.mockImplementation(() => {
        callOrder.push('reload');
        return Promise.resolve();
      });
      mockStore.get.mockImplementation(() => {
        callOrder.push('get');
        return Promise.resolve(null);
      });

      await loadSettings();

      expect(callOrder[0]).toBe('reload');
      expect(callOrder[1]).toBe('get');
    });

    it('should return stored settings when available', async () => {
      const { loadSettings } = await importModule();
      mockStore.get.mockResolvedValue({
        fontSize: 16,
        startupCommand: 'claude',
      });

      const result = await loadSettings();

      expect(result.fontSize).toBe(16);
      expect(result.startupCommand).toBe('claude');
    });

    it('should fill missing fields with defaults for partial settings', async () => {
      const { loadSettings } = await importModule();
      mockStore.get.mockResolvedValue({ fontSize: 18 });

      const result = await loadSettings();

      expect(result.fontSize).toBe(18);
      expect(result.startupCommand).toBe('none');
    });

    it('should return default settings when store throws an error', async () => {
      const { loadSettings } = await importModule();
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockStore.reload.mockRejectedValue(new Error('Store error'));

      const result = await loadSettings();

      expect(result.fontSize).toBe(13);
      expect(result.startupCommand).toBe('none');
      errorSpy.mockRestore();
    });
  });

  describe('saveSettings', () => {
    it('should save settings with the globalSettings key', async () => {
      const { saveSettings } = await importModule();
      const settings = { fontSize: 16, startupCommand: 'claude' as const };

      await saveSettings(settings);

      expect(mockStore.set).toHaveBeenCalledWith('globalSettings', settings);
      expect(mockStore.save).toHaveBeenCalled();
    });

    it('should not throw when store save fails', async () => {
      const { saveSettings } = await importModule();
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockStore.save.mockRejectedValue(new Error('Save error'));

      await expect(saveSettings({ fontSize: 13, startupCommand: 'none' })).resolves.not.toThrow();
      errorSpy.mockRestore();
    });
  });

  describe('loadProjectSettings', () => {
    it('should return default project settings when no settings are stored', async () => {
      const { loadProjectSettings } = await importModule();
      mockStore.get.mockResolvedValue(null);

      const result = await loadProjectSettings('/path/to/project');

      expect(result.searchExcludePatterns).toEqual(DEFAULT_EXCLUDE_PATTERNS);
      expect(result.worktreeCopyPatterns).toEqual([]);
      expect(result.worktreeInitCommands).toEqual([]);
    });

    it('should normalize project path for store key', async () => {
      const { loadProjectSettings } = await importModule();
      mockStore.get.mockResolvedValue(null);

      await loadProjectSettings('/Users/test/project');

      expect(mockStore.get).toHaveBeenCalledWith('project__Users_test_project');
    });

    it('should return stored project settings', async () => {
      const { loadProjectSettings } = await importModule();
      const storedSettings = {
        searchExcludePatterns: ['*.log'],
        worktreeCopyPatterns: ['**/.env*', 'config.json'],
        worktreeInitCommands: [
          { name: 'Install', command: 'npm install', enabled: true, auto: true },
        ],
      };
      mockStore.get.mockResolvedValue(storedSettings);

      const result = await loadProjectSettings('/path/to/project');

      expect(result.searchExcludePatterns).toEqual(['*.log']);
      expect(result.worktreeCopyPatterns).toEqual(['**/.env*', 'config.json']);
      expect(result.worktreeInitCommands).toHaveLength(1);
    });

    it('should fill missing fields with defaults for partial settings', async () => {
      const { loadProjectSettings } = await importModule();
      mockStore.get.mockResolvedValue({ searchExcludePatterns: ['*.tmp'] });

      const result = await loadProjectSettings('/path/to/project');

      expect(result.searchExcludePatterns).toEqual(['*.tmp']);
      expect(result.worktreeCopyPatterns).toEqual([]);
      expect(result.worktreeInitCommands).toEqual([]);
    });

    it('should preserve portConfig and composeIsolationConfig if present', async () => {
      const { loadProjectSettings } = await importModule();
      const storedSettings = {
        searchExcludePatterns: [],
        worktreeCopyPatterns: [],
        worktreeInitCommands: [],
        portConfig: {
          enabled: true,
          worktreeAssignments: {},
          targetFiles: ['.env*'],
        },
        composeIsolationConfig: {
          enabled: true,
          disabledFiles: [],
        },
      };
      mockStore.get.mockResolvedValue(storedSettings);

      const result = await loadProjectSettings('/path/to/project');

      expect(result.portConfig).toEqual(storedSettings.portConfig);
      expect(result.composeIsolationConfig).toEqual(storedSettings.composeIsolationConfig);
    });

    it('should return default settings when store throws an error', async () => {
      const { loadProjectSettings } = await importModule();
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockStore.reload.mockRejectedValue(new Error('Store error'));

      const result = await loadProjectSettings('/path/to/project');

      expect(result.searchExcludePatterns).toEqual(DEFAULT_EXCLUDE_PATTERNS);
      errorSpy.mockRestore();
    });
  });

  describe('saveProjectSettings', () => {
    it('should save project settings with normalized path key', async () => {
      const { saveProjectSettings } = await importModule();
      const settings = {
        searchExcludePatterns: ['*.log'],
        worktreeCopyPatterns: [],
        worktreeInitCommands: [],
      };

      await saveProjectSettings('/Users/test/project', settings);

      expect(mockStore.set).toHaveBeenCalledWith('project__Users_test_project', settings);
      expect(mockStore.save).toHaveBeenCalled();
    });

    it('should not throw when store save fails', async () => {
      const { saveProjectSettings } = await importModule();
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockStore.save.mockRejectedValue(new Error('Save error'));

      await expect(
        saveProjectSettings('/path', {
          searchExcludePatterns: [],
          worktreeCopyPatterns: [],
          worktreeInitCommands: [],
        })
      ).resolves.not.toThrow();
      errorSpy.mockRestore();
    });
  });
});
