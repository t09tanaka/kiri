import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/svelte';
import RecentProjectItem from './RecentProjectItem.svelte';

describe('RecentProjectItem Component (Browser)', () => {
  const mockProject = {
    name: 'my-project',
    path: '/Users/testuser/Documents/my-project',
    lastOpened: Date.now() - 1000 * 60 * 30, // 30 minutes ago
    gitBranch: 'main',
  };

  const mockProjectNoGit = {
    name: 'simple-folder',
    path: '/Users/testuser/Documents/simple-folder',
    lastOpened: Date.now() - 1000 * 60 * 60 * 24 * 3, // 3 days ago
    gitBranch: null,
  };

  afterEach(() => {
    cleanup();
  });

  it('renders project name', () => {
    render(RecentProjectItem, {
      props: { project: mockProject, onSelect: vi.fn(), onRemove: vi.fn() },
    });

    expect(screen.getByText('my-project')).toBeInTheDocument();
  });

  it('renders shortened path with ~ for home directory', () => {
    render(RecentProjectItem, {
      props: { project: mockProject, onSelect: vi.fn(), onRemove: vi.fn() },
    });

    expect(screen.getByText('~/Documents/my-project')).toBeInTheDocument();
  });

  it('renders time ago for recently opened projects', () => {
    render(RecentProjectItem, {
      props: { project: mockProject, onSelect: vi.fn(), onRemove: vi.fn() },
    });

    expect(screen.getByText('30m ago')).toBeInTheDocument();
  });

  it('renders time ago in days for older projects', () => {
    render(RecentProjectItem, {
      props: { project: mockProjectNoGit, onSelect: vi.fn(), onRemove: vi.fn() },
    });

    expect(screen.getByText('3d ago')).toBeInTheDocument();
  });

  it('calls onSelect when item is clicked', async () => {
    const onSelect = vi.fn();
    const { container } = render(RecentProjectItem, {
      props: { project: mockProject, onSelect, onRemove: vi.fn() },
    });

    const item = container.querySelector('.project-item');
    await fireEvent.click(item!);

    expect(onSelect).toHaveBeenCalled();
  });

  it('calls onSelect when Enter key is pressed', async () => {
    const onSelect = vi.fn();
    const { container } = render(RecentProjectItem, {
      props: { project: mockProject, onSelect, onRemove: vi.fn() },
    });

    const item = container.querySelector('.project-item');
    await fireEvent.keyDown(item!, { key: 'Enter' });

    expect(onSelect).toHaveBeenCalled();
  });

  it('calls onSelect when Space key is pressed', async () => {
    const onSelect = vi.fn();
    const { container } = render(RecentProjectItem, {
      props: { project: mockProject, onSelect, onRemove: vi.fn() },
    });

    const item = container.querySelector('.project-item');
    await fireEvent.keyDown(item!, { key: ' ' });

    expect(onSelect).toHaveBeenCalled();
  });

  it('has remove button', () => {
    render(RecentProjectItem, {
      props: { project: mockProject, onSelect: vi.fn(), onRemove: vi.fn() },
    });

    expect(screen.getByTitle('Remove from recent')).toBeInTheDocument();
  });

  it('calls onRemove when remove button is clicked', async () => {
    const onRemove = vi.fn();
    render(RecentProjectItem, {
      props: { project: mockProject, onSelect: vi.fn(), onRemove },
    });

    const removeButton = screen.getByTitle('Remove from recent');
    await fireEvent.click(removeButton);

    expect(onRemove).toHaveBeenCalled();
  });

  it('does not call onSelect when remove button is clicked', async () => {
    const onSelect = vi.fn();
    const onRemove = vi.fn();
    render(RecentProjectItem, {
      props: { project: mockProject, onSelect, onRemove },
    });

    const removeButton = screen.getByTitle('Remove from recent');
    await fireEvent.click(removeButton);

    expect(onRemove).toHaveBeenCalled();
    expect(onSelect).not.toHaveBeenCalled();
  });

  it('has project icon', () => {
    const { container } = render(RecentProjectItem, {
      props: { project: mockProject, onSelect: vi.fn(), onRemove: vi.fn() },
    });

    const icon = container.querySelector('.project-icon');
    expect(icon).toBeInTheDocument();
  });

  it('renders just now for very recent projects', () => {
    const recentProject = {
      ...mockProject,
      lastOpened: Date.now() - 1000 * 10, // 10 seconds ago
    };
    render(RecentProjectItem, {
      props: { project: recentProject, onSelect: vi.fn(), onRemove: vi.fn() },
    });

    expect(screen.getByText('just now')).toBeInTheDocument();
  });
});
