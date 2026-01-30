import { describe, it, expect } from 'vitest';
import { branchToWorktreeName } from './gitWorktree';

describe('branchToWorktreeName', () => {
  it('replaces single slash with hyphen', () => {
    expect(branchToWorktreeName('features/admin')).toBe('features-admin');
  });

  it('replaces multiple slashes with hyphens', () => {
    expect(branchToWorktreeName('features/admin/settings')).toBe('features-admin-settings');
  });

  it('keeps branch name without slashes unchanged', () => {
    expect(branchToWorktreeName('fix-bug')).toBe('fix-bug');
  });

  it('handles empty string', () => {
    expect(branchToWorktreeName('')).toBe('');
  });

  it('handles main branch', () => {
    expect(branchToWorktreeName('main')).toBe('main');
  });

  it('handles branch with leading slash', () => {
    expect(branchToWorktreeName('/feature')).toBe('-feature');
  });

  it('handles branch with trailing slash', () => {
    expect(branchToWorktreeName('feature/')).toBe('feature-');
  });

  it('handles consecutive slashes', () => {
    expect(branchToWorktreeName('a//b')).toBe('a--b');
  });
});
