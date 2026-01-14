import { invoke } from '@tauri-apps/api/core';
import type { GitRepoInfo, GitFileDiff } from '@/lib/stores/gitStore';

/**
 * Git operations service
 * Wraps Tauri git commands for testability
 */
export const gitService = {
  /**
   * Get git status for a repository
   */
  getStatus: (path: string): Promise<GitRepoInfo> => invoke('get_git_status', { path }),

  /**
   * Get git diff for a specific file
   * Returns unified diff format
   */
  getFileDiff: (repoPath: string, filePath: string): Promise<string> =>
    invoke('get_git_diff', { repoPath, filePath }),

  /**
   * Get all git diffs for a repository
   */
  getAllDiffs: (repoPath: string): Promise<GitFileDiff[]> =>
    invoke('get_all_git_diffs', { repoPath }),
};
