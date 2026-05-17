import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the Tauri command surface before importing windowService so the
// service's `invoke` import binds to the mock.
const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

// `windowService.setTitle` / `setSizeAndCenter` reach into
// `getCurrentWindow()`, but the tests in this file don't touch those
// methods, so a minimal stub is enough.
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    setTitle: vi.fn(),
    setSize: vi.fn(),
    center: vi.fn(),
  }),
  LogicalSize: class {
    constructor(
      public width: number,
      public height: number
    ) {}
  },
}));

import { windowService } from './windowService';

describe('windowService multi-window concurrency', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('createWindow forwards projectPath to the create_window command', async () => {
    invokeMock.mockResolvedValue(undefined);
    await windowService.createWindow({ projectPath: '/p/one' });
    expect(invokeMock).toHaveBeenCalledWith('create_window', { projectPath: '/p/one' });
  });

  it('createWindow defaults projectPath to null when omitted', async () => {
    invokeMock.mockResolvedValue(undefined);
    await windowService.createWindow();
    expect(invokeMock).toHaveBeenCalledWith('create_window', { projectPath: null });
  });

  it('focusOrCreateWindow forwards the path and returns the backend boolean', async () => {
    invokeMock.mockResolvedValue(true);
    const result = await windowService.focusOrCreateWindow('/p/two');
    expect(invokeMock).toHaveBeenCalledWith('focus_or_create_window', {
      projectPath: '/p/two',
    });
    expect(result).toBe(true);
  });

  it('N concurrent createWindow calls all reach the backend without lock contention', async () => {
    invokeMock.mockResolvedValue(undefined);

    const calls = Array.from({ length: 16 }, (_, i) =>
      windowService.createWindow({ projectPath: `/p/${i}` })
    );
    await Promise.all(calls);

    expect(invokeMock).toHaveBeenCalledTimes(16);
    // Order of arrival is not deterministic across microtasks, but
    // every projectPath we sent must appear exactly once.
    const seen = new Set(
      invokeMock.mock.calls.map((args) => (args[1] as { projectPath: string }).projectPath)
    );
    for (let i = 0; i < 16; i++) {
      expect(seen.has(`/p/${i}`)).toBe(true);
    }
  });

  it('concurrent focusOrCreateWindow on same path each receive an independent result', async () => {
    // Simulates two clicks on the same recent-project entry. The
    // backend serialises through the registry mutex and decides which
    // is the create vs the focus; the service must surface whatever
    // the backend returns to each caller independently.
    invokeMock
      .mockResolvedValueOnce(false) // first caller created the window
      .mockResolvedValueOnce(true); // second caller focused the existing one

    const [a, b] = await Promise.all([
      windowService.focusOrCreateWindow('/shared'),
      windowService.focusOrCreateWindow('/shared'),
    ]);
    expect([a, b].sort()).toEqual([false, true]);
    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it('unregisterWindow on a label that never registered still forwards', async () => {
    // The backend tolerates unregistering an unknown label; verify
    // the JS side doesn't pre-filter and silently drop the call.
    invokeMock.mockResolvedValue(undefined);
    await windowService.unregisterWindow('main');
    expect(invokeMock).toHaveBeenCalledWith('unregister_window', { label: 'main' });
  });
});
