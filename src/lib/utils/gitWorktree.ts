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

/** Maximum allowed branch name length */
const MAX_BRANCH_NAME_LENGTH = 200;

/**
 * Validate a branch name according to Git ref naming rules.
 * Returns an error message string if invalid, or null if valid.
 *
 * Rules enforced (subset of git-check-ref-format):
 * - No spaces, ~, ^, :, ?, *, [, \ or control characters
 * - Cannot start with '.' or '-'
 * - Cannot end with '.lock'
 * - Cannot contain '..'
 * - Max length 200 characters
 */
export function validateBranchName(name: string): string | null {
  if (name.length > MAX_BRANCH_NAME_LENGTH) {
    return `Branch name must be ${MAX_BRANCH_NAME_LENGTH} characters or fewer.`;
  }

  if (name.startsWith('.') || name.startsWith('-')) {
    return "Branch name cannot start with '.' or '-'.";
  }

  if (name.endsWith('.lock')) {
    return "Branch name cannot end with '.lock'.";
  }

  if (name.includes('..')) {
    return "Branch name cannot contain '..'.";
  }

  // Check for invalid characters: space, ~, ^, :, ?, *, [, \, control chars
  for (const char of name) {
    const code = char.charCodeAt(0);
    if (code < 32 || code === 127) {
      return 'Branch name contains invalid character: control character.';
    }
  }

  const invalidCharMatch = name.match(/[\s~^:?*[\\\]]/);
  if (invalidCharMatch) {
    return `Branch name contains invalid character: '${invalidCharMatch[0]}'.`;
  }

  return null;
}
