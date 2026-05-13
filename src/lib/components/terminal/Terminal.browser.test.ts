import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, test, vi } from 'vitest';

// Mock services used by Terminal.svelte beyond what browser-setup.ts already covers.
// The pane-label markup is rendered synchronously from props, so the async onMount path
// (xterm init, PTY creation) can safely fail or no-op without affecting assertions.
vi.mock('@/lib/services/gitService', () => ({
  gitService: {
    getWorktreeInfo: vi.fn().mockResolvedValue(null),
  },
}));

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
