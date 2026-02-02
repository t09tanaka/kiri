import { invoke } from '@tauri-apps/api/core';

export interface WorktreeInfo {
  name: string;
  path: string;
  branch: string | null;
  is_locked: boolean;
  is_main: boolean;
  is_valid: boolean;
}

export interface WorktreeContext {
  is_worktree: boolean;
  main_repo_path: string | null;
  worktree_name: string | null;
}

export interface BranchInfo {
  name: string;
  is_head: boolean;
  /** Unix timestamp (seconds) of the last commit on this branch */
  last_commit_time: number | null;
}

export interface CopyResult {
  copied_files: string[];
  skipped_files: string[];
  errors: string[];
}

/**
 * Git worktree operations service
 * Wraps Tauri worktree commands for testability
 */
export const worktreeService = {
  /**
   * List all worktrees for a repository
   */
  list: (repoPath: string): Promise<WorktreeInfo[]> => invoke('list_worktrees', { repoPath }),

  /**
   * Create a new worktree
   */
  create: (
    repoPath: string,
    name: string,
    branch: string | null,
    newBranch: boolean
  ): Promise<WorktreeInfo> => invoke('create_worktree', { repoPath, name, branch, newBranch }),

  /**
   * Remove a worktree by name
   */
  remove: (repoPath: string, name: string): Promise<void> =>
    invoke('remove_worktree', { repoPath, name }),

  /**
   * Get worktree context for a repository path
   */
  getContext: (repoPath: string): Promise<WorktreeContext> =>
    invoke('get_worktree_context', { repoPath }),

  /**
   * List local branches for a repository
   */
  listBranches: (repoPath: string): Promise<BranchInfo[]> => invoke('list_branches', { repoPath }),

  /**
   * Copy files matching patterns from source to target directory
   */
  copyFiles: (sourcePath: string, targetPath: string, patterns: string[]): Promise<CopyResult> =>
    invoke('copy_files_to_worktree', { sourcePath, targetPath, patterns }),
};
