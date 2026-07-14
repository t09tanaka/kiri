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
  getStartupCommandString: vi.fn().mockReturnValue(''),
}));

vi.mock('@/lib/services/filePathLinkProvider', () => ({
  createFilePathLinkProvider: vi.fn().mockReturnValue({
    provideLinks: vi.fn(),
  }),
}));

// Extend the terminalService mock from browser-setup.ts so the Terminal
// component has the service methods it calls during mount.
vi.mock('@/lib/services/terminalService', () => ({
  terminalService: {
    createTerminal: vi.fn().mockResolvedValue(1),
    writeTerminal: vi.fn().mockResolvedValue(undefined),
    resizeTerminal: vi.fn().mockResolvedValue(undefined),
    closeTerminal: vi.fn().mockResolvedValue(undefined),
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

describe('Terminal controls in narrow panes (regression: close button overflows the pane)', () => {
  afterEach(() => {
    cleanup();
  });

  // Splitting a row of terminals more than ~4 ways makes each pane narrower
  // than the header's natural content width. The header must shed its optional
  // content instead of pushing the close button past the pane's right edge,
  // where `overflow: hidden` on the wrapper clips it away.
  async function renderAtWidth(width: number) {
    const { container } = render(Terminal, {
      props: {
        paneId: 'p1',
        name: 'a-rather-long-pane-name',
        color: 'jade',
        showControls: true,
        onSplitVertical: () => {},
        onSplitHorizontal: () => {},
        onMinimize: () => {},
        onClose: () => {},
      },
    });
    container.style.width = `${width}px`;
    container.style.height = '240px';
    await tick();
    return container;
  }

  test.each([320, 200, 170, 150, 120, 100, 72, 60, 44])(
    'close button stays inside the pane at %ipx wide',
    async (width) => {
      const container = await renderAtWidth(width);
      const wrapper = container.querySelector('.terminal-wrapper') as HTMLElement;
      const closeBtn = container.querySelector('.close-btn') as HTMLElement;

      const wrapperRect = wrapper.getBoundingClientRect();
      const closeRect = closeBtn.getBoundingClientRect();

      expect(closeRect.width).toBeGreaterThan(0);
      // Sub-pixel tolerance: layout rounding, not overflow.
      expect(closeRect.right).toBeLessThanOrEqual(wrapperRect.right + 0.5);
      expect(closeRect.left).toBeGreaterThanOrEqual(wrapperRect.left - 0.5);
    }
  );

  // A container size query measures the content box, so each breakpoint bites
  // 16px (the header's horizontal padding) later than its declared width. These
  // pin the widths where controls actually drop out, in pane terms.
  test.each([
    { width: 320, name: true, split: true, minimize: true },
    { width: 210, name: false, split: true, minimize: true },
    { width: 160, name: false, split: false, minimize: true },
    { width: 80, name: false, split: false, minimize: false },
  ])(
    'at $widthpx: name=$name split=$split minimize=$minimize',
    async ({ width, name, split, minimize }) => {
      const container = await renderAtWidth(width);
      const shown = (selector: string) => {
        const el = container.querySelector(selector) as HTMLElement;
        return getComputedStyle(el).display !== 'none';
      };

      expect(shown('.pane-name')).toBe(name);
      expect(shown('.split-btn')).toBe(split);
      expect(shown('.minimize-btn')).toBe(minimize);
      // Close is the one control that must never drop out.
      expect(shown('.close-btn')).toBe(true);
    }
  );
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
