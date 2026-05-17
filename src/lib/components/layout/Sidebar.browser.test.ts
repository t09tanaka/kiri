import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';

// Sidebar embeds FileTree, which subscribes to gitStore + projectStore
// and pokes fileService.readDirectory on mount. browser-setup.ts
// already mocks fileService.readDirectory to return [] so the tree
// mounts empty. We only assert on the Sidebar's own DOM, never the
// FileTree internals.
vi.mock('@/lib/services/gitService', () => ({
  gitService: {
    getStatus: vi.fn().mockResolvedValue({ root: '', statuses: [], branch: null }),
    getAllDiffs: vi.fn().mockResolvedValue([]),
  },
}));

// FileTree's drag-and-drop setup pokes the per-window listener;
// browser-setup.ts's eventService mock only stubs `listen`/`emit`, so
// we extend it here to silence "is not a function" rejections.
vi.mock('@/lib/services/eventService', () => ({
  eventService: {
    listen: vi.fn().mockResolvedValue(() => {}),
    listenCurrentWindow: vi.fn().mockResolvedValue(() => {}),
    emit: vi.fn().mockResolvedValue(undefined),
  },
}));

import Sidebar from './Sidebar.svelte';

describe('Sidebar', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the Explorer header', () => {
    const { container } = render(Sidebar, { props: { rootPath: '' } });
    const aside = container.querySelector('aside[data-testid="sidebar"]');
    expect(aside).not.toBeNull();
    expect(aside?.querySelector('.title')?.textContent).toBe('Explorer');
  });

  it('applies the width prop to inline style', () => {
    const { container } = render(Sidebar, { props: { width: 320, rootPath: '' } });
    const aside = container.querySelector('aside[data-testid="sidebar"]') as HTMLElement;
    expect(aside.style.width).toBe('320px');
  });

  it('falls back to the default width when omitted', () => {
    const { container } = render(Sidebar, { props: { rootPath: '' } });
    const aside = container.querySelector('aside[data-testid="sidebar"]') as HTMLElement;
    expect(aside.style.width).toBe('250px');
  });

  it('renders the folder icon next to the title', () => {
    const { container } = render(Sidebar, { props: { rootPath: '' } });
    const icon = container.querySelector('.sidebar-header .header-icon svg');
    expect(icon).not.toBeNull();
  });

  it('mounts a FileTree slot inside sidebar-content', () => {
    const { container } = render(Sidebar, { props: { rootPath: '' } });
    const content = container.querySelector('.sidebar-content');
    expect(content).not.toBeNull();
    // FileTree adds at least one descendant element when it mounts.
    // We don't assert the precise FileTree DOM here - that belongs in
    // FileTree's own test - we just verify the slot is populated.
    expect(content?.childElementCount).toBeGreaterThan(0);
  });
});
