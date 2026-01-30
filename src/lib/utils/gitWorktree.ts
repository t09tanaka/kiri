/**
 * Git worktree utility functions
 */

/**
 * Convert branch name to worktree name.
 * Replaces '/' with '-' since directory names cannot contain '/'.
 *
 * @example
 * branchToWorktreeName('features/admin') // => 'features-admin'
 * branchToWorktreeName('fix-bug') // => 'fix-bug'
 */
export function branchToWorktreeName(branchName: string): string {
  return branchName.replace(/\//g, '-');
}
