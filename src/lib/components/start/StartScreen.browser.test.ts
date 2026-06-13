import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, fireEvent, cleanup } from '@testing-library/svelte';
import { readable } from 'svelte/store';

// Keep the real constants/helpers (STARTUP_COMMANDS, defaults) but spy on
// the disk-write so we can assert the selection is persisted immediately.
// NOTE: In browser tests vi.mock is hoisted, so the factory cannot close over
// top-level variables.  We use vi.hoisted() to create the spy before hoisting.
const { saveSettings } = vi.hoisted(() => ({
  saveSettings: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@/lib/services/persistenceService', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/services/persistenceService')>();
  return { ...actual, saveSettings };
});

// projectStore pulls in Tauri APIs; stub the surface StartScreen touches.
vi.mock('@/lib/stores/projectStore', () => ({
  projectStore: {
    subscribe: readable({ currentPath: null, recentProjects: [] }).subscribe,
    openProject: vi.fn(),
    removeProject: vi.fn(),
  },
  recentProjects: readable([]),
}));

import StartScreen from './StartScreen.svelte';
import { settingsStore } from '@/lib/stores/settingsStore';

describe('StartScreen — startup command persistence (Browser)', () => {
  beforeEach(() => {
    saveSettings.mockClear();
    settingsStore.setStartupCommand('none');
  });

  afterEach(() => {
    cleanup();
  });

  it('persists the startup command to disk immediately when clicked', async () => {
    const { getByText } = render(StartScreen);

    await fireEvent.click(getByText('Claude'));

    // The fix: do not wait on the 500ms auto-persist window — save now.
    expect(saveSettings).toHaveBeenCalledTimes(1);
    expect(saveSettings.mock.calls[0][0]).toMatchObject({ startupCommand: 'claude' });
    expect(settingsStore.getStateForPersistence().startupCommand).toBe('claude');
  });

  it('persists each subsequent selection', async () => {
    const { getByText } = render(StartScreen);

    await fireEvent.click(getByText('Codex'));
    await fireEvent.click(getByText('None'));

    expect(saveSettings).toHaveBeenCalledTimes(2);
    expect(saveSettings.mock.calls[0][0]).toMatchObject({ startupCommand: 'codex' });
    expect(saveSettings.mock.calls[1][0]).toMatchObject({ startupCommand: 'none' });
  });
});
