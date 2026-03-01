import { describe, it, expect } from 'vitest';
import { branchToWorktreeName, validateBranchName } from './gitWorktree';

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

describe('validateBranchName', () => {
  it('returns null for valid branch names', () => {
    expect(validateBranchName('feature/my-branch')).toBeNull();
    expect(validateBranchName('fix-bug')).toBeNull();
    expect(validateBranchName('release/v1.0.0')).toBeNull();
    expect(validateBranchName('a')).toBeNull();
  });

  it('rejects names longer than 200 characters', () => {
    const longName = 'a'.repeat(201);
    expect(validateBranchName(longName)).toContain('200 characters');
    expect(validateBranchName('a'.repeat(200))).toBeNull();
  });

  it('rejects names starting with a dot', () => {
    expect(validateBranchName('.hidden')).toContain("'.' or '-'");
  });

  it('rejects names starting with a hyphen', () => {
    expect(validateBranchName('-bad')).toContain("'.' or '-'");
  });

  it('rejects names ending with .lock', () => {
    expect(validateBranchName('branch.lock')).toContain('.lock');
  });

  it('allows names containing lock but not ending with .lock', () => {
    expect(validateBranchName('lock-branch')).toBeNull();
    expect(validateBranchName('branch.lock.bak')).toBeNull();
  });

  it('rejects names containing double dots', () => {
    expect(validateBranchName('a..b')).toContain("'..'");
  });

  it('rejects names containing spaces', () => {
    expect(validateBranchName('my branch')).toContain('invalid character');
  });

  it('rejects names containing tilde', () => {
    expect(validateBranchName('branch~1')).toContain("'~'");
  });

  it('rejects names containing caret', () => {
    expect(validateBranchName('branch^2')).toContain("'^'");
  });

  it('rejects names containing colon', () => {
    expect(validateBranchName('branch:name')).toContain("':'");
  });

  it('rejects names containing question mark', () => {
    expect(validateBranchName('branch?')).toContain("'?'");
  });

  it('rejects names containing asterisk', () => {
    expect(validateBranchName('branch*')).toContain("'*'");
  });

  it('rejects names containing open bracket', () => {
    expect(validateBranchName('branch[0]')).toContain("'['");
  });

  it('rejects names containing backslash', () => {
    expect(validateBranchName('branch\\name')).toContain("'\\'");
  });

  it('rejects names containing control characters', () => {
    expect(validateBranchName('branch\x00name')).toContain('control character');
    expect(validateBranchName('branch\x1fname')).toContain('control character');
    expect(validateBranchName('branch\x7fname')).toContain('control character');
  });
});
