import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { gitService } from './gitService';

describe('gitService.getWorktreeInfo', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('invokes get_worktree_info with the given path', async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    await gitService.getWorktreeInfo('/some/path');

    expect(invoke).toHaveBeenCalledWith('get_worktree_info', { path: '/some/path' });
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it('returns null when path is not in a git repo', async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    const result = await gitService.getWorktreeInfo('/nope');

    expect(result).toBeNull();
  });

  it('returns WorktreeInfo when path is inside a linked worktree', async () => {
    vi.mocked(invoke).mockResolvedValue({
      is_linked_worktree: true,
      name: 'feat-foo',
      root: '/tmp/wt/feat-foo',
    });

    const result = await gitService.getWorktreeInfo('/tmp/wt/feat-foo/src');

    expect(result).toEqual({
      is_linked_worktree: true,
      name: 'feat-foo',
      root: '/tmp/wt/feat-foo',
    });
  });

  it('returns WorktreeInfo with is_linked_worktree false when in main worktree', async () => {
    vi.mocked(invoke).mockResolvedValue({
      is_linked_worktree: false,
      name: 'kiri',
      root: '/Users/tanakatakuto/Documents/GitHub/kiri',
    });

    const result = await gitService.getWorktreeInfo('/Users/tanakatakuto/Documents/GitHub/kiri');

    expect(result).toEqual({
      is_linked_worktree: false,
      name: 'kiri',
      root: '/Users/tanakatakuto/Documents/GitHub/kiri',
    });
  });
});
