import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { tick } from 'svelte';
import { terminalStore } from '@/lib/stores/terminalStore';
import { terminalRegistry } from '@/lib/stores/terminalRegistry';

// Mock services used by Terminal.svelte beyond what browser-setup.ts already covers.
// The pane-label markup is rendered synchronously from props, so the async onMount path
// (xterm init, PTY creation) can safely fail or no-op without affecting assertions.
vi.mock('@/lib/services/openerService', () => ({
  openerService: {
    openUrl: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock('@/lib/services/notificationService', () => ({
  notificationService: {
    init: vi.fn(),
    parseNotifications: vi.fn().mockReturnValue({ output: '', notifications: [] }),
    notify: vi.fn(),
  },
}));

vi.mock('@/lib/services/persistenceService', () => ({
  DEFAULT_STARTUP_COMMAND: 'none',
  loadShortcuts: vi.fn().mockResolvedValue([]),
  saveShortcuts: vi.fn().mockResolvedValue(undefined),
  loadNumberRowEnabled: vi.fn().mockResolvedValue(false),
  saveNumberRowEnabled: vi.fn().mockResolvedValue(undefined),
  getStartupCommandString: vi.fn().mockReturnValue(''),
}));

vi.mock('@/lib/services/filePathLinkProvider', () => ({
  createFilePathLinkProvider: vi.fn().mockReturnValue({
    provideLinks: vi.fn(),
  }),
}));

// Extend the terminalService mock from browser-setup.ts so the process-info poller
// has the methods it expects (called via setTimeout after mount).
vi.mock('@/lib/services/terminalService', () => ({
  terminalService: {
    createTerminal: vi.fn().mockResolvedValue(1),
    writeTerminal: vi.fn().mockResolvedValue(undefined),
    resizeTerminal: vi.fn().mockResolvedValue(undefined),
    closeTerminal: vi.fn().mockResolvedValue(undefined),
    getProcessInfo: vi.fn().mockResolvedValue({ name: '' }),
    getCwd: vi.fn().mockResolvedValue(null),
  },
}));

import Terminal from './Terminal.svelte';

describe('Terminal pane-label header', () => {
  afterEach(() => {
    cleanup();
  });

  test('renders pane-name when name is set', () => {
    const { container } = render(Terminal, {
      props: { paneId: 'p1', name: 'build' },
    });
    expect(container.querySelector('.pane-name')?.textContent).toBe('build');
    expect(container.querySelector('.pane-dot')).toBeNull();
  });

  test('renders pane-dot with color variable when color is set', () => {
    const { container } = render(Terminal, {
      props: { paneId: 'p1', color: 'jade' },
    });
    const label = container.querySelector('.pane-label') as HTMLElement;
    expect(label).not.toBeNull();
    expect(label.style.getPropertyValue('--pane-color')).toBe('var(--pane-color-jade)');
    expect(container.querySelector('.pane-dot')).not.toBeNull();
    expect(container.querySelector('.pane-name')).toBeNull();
  });

  test('renders both dot and name when both are set', () => {
    const { container } = render(Terminal, {
      props: { paneId: 'p1', name: 'agent', color: 'iris' },
    });
    expect(container.querySelector('.pane-dot')).not.toBeNull();
    expect(container.querySelector('.pane-name')?.textContent).toBe('agent');
  });

  test('omits pane-label entirely when neither name nor color is set', () => {
    const { container } = render(Terminal, { props: { paneId: 'p1' } });
    expect(container.querySelector('.pane-label')).toBeNull();
  });
});

describe('Terminal paneId stability (regression: split kills original pane)', () => {
  afterEach(() => {
    cleanup();
    terminalRegistry.clearAll();
    terminalStore.reset();
  });

  // The bug: TerminalContainer.svelte uses {#key pane.type} to destroy and
  // re-mount the Terminal subtree when a terminal pane is wrapped in a new
  // parent split. In Svelte 5, the destroying Terminal's reactive `paneId`
  // prop briefly took the parent split's id (e.g. "split-1") instead of its
  // own original id (e.g. "pane-1"). That made paneExistsInStore() return
  // false and the destroy handler take the "true close" branch — killing the
  // still-needed PTY for the original pane.
  //
  // Fix: Terminal.svelte captures paneId once at construction and uses that
  // captured value in onDestroy. This test forces the prop to mutate after
  // mount and confirms the destroy path still uses the original id.
  test('onDestroy uses the paneId captured at mount, not the reactive prop value', async () => {
    const originalPaneId = 'pane-regression';
    const wrongPaneId = 'split-regression';

    // Empty tree → paneExistsInStore() returns false → destroy takes the
    // cleanup branch, which is where the captured-vs-reactive paneId matters.
    terminalStore.reset();

    const removeSpy = vi.spyOn(terminalRegistry, 'remove');

    const { rerender, unmount } = render(Terminal, {
      props: { paneId: originalPaneId },
    });
    await tick();

    // Simulate the parent tree restructure that the {#key pane.type} block
    // produces: the same component's prop briefly receives the split's id.
    await rerender({ paneId: wrongPaneId });
    await tick();

    unmount();
    await tick();

    // With the fix in place we must see the captured (original) id in the
    // destroy path. Without the fix this would be the wrong (split) id.
    expect(removeSpy).toHaveBeenCalledWith(originalPaneId);
    expect(removeSpy).not.toHaveBeenCalledWith(wrongPaneId);

    removeSpy.mockRestore();
  });
});
