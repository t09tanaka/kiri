import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, cleanup, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';

// gitService is not stubbed in browser-setup.ts, but FileTree subscribes to
// gitStore which would otherwise hit Tauri on mount.
vi.mock('@/lib/services/gitService', () => ({
  gitService: {
    getStatus: vi.fn().mockResolvedValue({ root: '', statuses: [], branch: null }),
    getAllDiffs: vi.fn().mockResolvedValue([]),
  },
}));

// browser-setup.ts only stubs `listen` + `emit`; FileTree also calls
// `listenCurrentWindow` for Tauri drag events.
vi.mock('@/lib/services/eventService', () => ({
  eventService: {
    listen: vi.fn().mockResolvedValue(() => {}),
    listenCurrentWindow: vi.fn().mockResolvedValue(() => {}),
    emit: vi.fn().mockResolvedValue(undefined),
  },
}));

// fileService.createDirectory / deletePath are referenced via FileTreeItem;
// stub them to prevent real `invoke` calls if a test exercises that path.
vi.mock('@/lib/services/fileService', () => ({
  fileService: {
    readFile: vi.fn().mockResolvedValue(''),
    writeFile: vi.fn().mockResolvedValue(undefined),
    readDirectory: vi.fn().mockResolvedValue([]),
    getHomeDirectory: vi.fn().mockResolvedValue('/home/user'),
    revealInFinder: vi.fn().mockResolvedValue(undefined),
    createDirectory: vi.fn().mockResolvedValue(''),
    deletePath: vi.fn().mockResolvedValue(undefined),
  },
}));

import { fileService } from '@/lib/services/fileService';
import FileTree from './FileTree.svelte';
import type { FileEntry } from './types';

const mockedReadDirectory = vi.mocked(fileService.readDirectory);

function entry(partial: Partial<FileEntry> & { name: string; path: string }): FileEntry {
  return {
    is_dir: false,
    is_hidden: false,
    is_gitignored: false,
    ...partial,
  };
}

// Helper: install a path -> entries map. FileTree calls readDirectory on
// both onMount and a separate $effect, so per-path resolution avoids the
// `mockResolvedValueOnce` exhaustion footgun.
function setReadDirectory(map: Record<string, FileEntry[]>) {
  mockedReadDirectory.mockImplementation(async (path: string) => map[path] ?? []);
}

describe('FileTree', () => {
  beforeEach(() => {
    mockedReadDirectory.mockReset();
    mockedReadDirectory.mockResolvedValue([]);
  });

  afterEach(() => {
    cleanup();
  });

  it('renders the wrapper with the file-tree testid', () => {
    const { container } = render(FileTree, { props: { rootPath: '/repo' } });
    const root = container.querySelector('[data-testid="file-tree"]');
    expect(root).not.toBeNull();
  });

  it('shows the loading skeleton on first paint, then resolves', async () => {
    let resolve!: (entries: FileEntry[]) => void;
    mockedReadDirectory.mockImplementation(
      () =>
        new Promise<FileEntry[]>((r) => {
          resolve = r;
        })
    );

    const { container } = render(FileTree, { props: { rootPath: '/repo' } });

    // Skeleton placeholders should render while readDirectory is pending.
    expect(container.querySelector('.loading-skeleton')).not.toBeNull();

    resolve([]);
    await waitFor(() => {
      expect(container.querySelector('.loading-skeleton')).toBeNull();
    });
  });

  it('renders the empty state when readDirectory resolves to []', async () => {
    setReadDirectory({ '/repo': [] });
    const { container } = render(FileTree, { props: { rootPath: '/repo' } });

    await waitFor(() => {
      expect(container.querySelector('.empty')?.textContent).toContain('Empty directory');
    });
  });

  it('extracts the project name from the trailing path segment', async () => {
    setReadDirectory({ '/path/to/kiri': [] });
    const { container } = render(FileTree, { props: { rootPath: '/path/to/kiri' } });

    await waitFor(() => {
      const header = container.querySelector('.project-header .project-name');
      expect(header?.textContent).toBe('kiri');
    });
  });

  it('hides children when the project header is collapsed', async () => {
    setReadDirectory({
      '/repo': [entry({ name: 'README.md', path: '/repo/README.md' })],
    });
    const { container } = render(FileTree, { props: { rootPath: '/repo' } });

    await waitFor(() => {
      expect(container.querySelectorAll('[data-drop-path]').length).toBe(1);
    });

    const header = container.querySelector('.project-header') as HTMLButtonElement;
    expect(header).not.toBeNull();
    expect(header.classList.contains('expanded')).toBe(true);

    header.click();
    await tick();

    expect(header.classList.contains('expanded')).toBe(false);
    expect(container.querySelectorAll('[data-drop-path]').length).toBe(0);
  });

  it('renders one tree item per entry returned by readDirectory', async () => {
    setReadDirectory({
      '/repo': [
        entry({ name: 'src', path: '/repo/src', is_dir: true }),
        entry({ name: 'README.md', path: '/repo/README.md' }),
        entry({ name: 'package.json', path: '/repo/package.json' }),
      ],
    });

    const { container } = render(FileTree, { props: { rootPath: '/repo' } });

    await waitFor(() => {
      const items = container.querySelectorAll('[data-drop-path]');
      expect(items.length).toBe(3);
    });

    const names = Array.from(container.querySelectorAll('.tree-item .name')).map(
      (n) => n.textContent
    );
    // Directories come first regardless of source order.
    expect(names[0]).toBe('src');
    expect(names).toContain('README.md');
    expect(names).toContain('package.json');
  });

  it('lazy-loads a directory entry on click by calling readDirectory with its path', async () => {
    setReadDirectory({
      '/repo': [entry({ name: 'src', path: '/repo/src', is_dir: true })],
      '/repo/src': [entry({ name: 'main.ts', path: '/repo/src/main.ts' })],
    });

    const { container } = render(FileTree, { props: { rootPath: '/repo' } });

    await waitFor(() => {
      expect(container.querySelector('.tree-item.directory')).not.toBeNull();
    });

    const dir = container.querySelector('.tree-item.directory') as HTMLButtonElement;
    dir.click();

    await waitFor(() => {
      expect(mockedReadDirectory).toHaveBeenCalledWith('/repo/src');
    });
    await waitFor(() => {
      const allNames = Array.from(container.querySelectorAll('.tree-item .name')).map(
        (n) => n.textContent
      );
      expect(allNames).toContain('main.ts');
    });
  });

  it('invokes onFileSelect with the file path when a file row is clicked', async () => {
    setReadDirectory({
      '/repo': [entry({ name: 'README.md', path: '/repo/README.md' })],
    });

    const onFileSelect = vi.fn();
    const { container } = render(FileTree, {
      props: { rootPath: '/repo', onFileSelect },
    });

    await waitFor(() => {
      expect(container.querySelector('.tree-item:not(.directory)')).not.toBeNull();
    });

    const fileBtn = container.querySelector('.tree-item:not(.directory)') as HTMLButtonElement;
    fileBtn.click();

    expect(onFileSelect).toHaveBeenCalledWith('/repo/README.md');
  });

  it('marks the clicked file as selected', async () => {
    setReadDirectory({
      '/repo': [entry({ name: 'README.md', path: '/repo/README.md' })],
    });

    const { container } = render(FileTree, { props: { rootPath: '/repo' } });

    await waitFor(() => {
      expect(container.querySelector('.tree-item:not(.directory)')).not.toBeNull();
    });

    const fileBtn = container.querySelector('.tree-item:not(.directory)') as HTMLButtonElement;
    fileBtn.click();
    await tick();

    expect(fileBtn.classList.contains('selected')).toBe(true);
  });

  it('triggers expand on Enter key for directories', async () => {
    setReadDirectory({
      '/repo': [entry({ name: 'src', path: '/repo/src', is_dir: true })],
      '/repo/src': [entry({ name: 'main.ts', path: '/repo/src/main.ts' })],
    });

    const { container } = render(FileTree, { props: { rootPath: '/repo' } });

    await waitFor(() => {
      expect(container.querySelector('.tree-item.directory')).not.toBeNull();
    });

    const dir = container.querySelector('.tree-item.directory') as HTMLButtonElement;
    dir.focus();
    dir.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })
    );

    await waitFor(() => {
      expect(mockedReadDirectory).toHaveBeenCalledWith('/repo/src');
    });
  });

  it('renders the error state when readDirectory rejects', async () => {
    mockedReadDirectory.mockRejectedValue(new Error('EACCES: permission denied'));

    const { container } = render(FileTree, { props: { rootPath: '/repo' } });

    await waitFor(() => {
      const err = container.querySelector('.error');
      expect(err).not.toBeNull();
      expect(err?.textContent).toContain('EACCES');
    });
  });
});
