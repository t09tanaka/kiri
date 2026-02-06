import { invoke } from '@tauri-apps/api/core';
import type { GitRepoInfo, GitFileDiff } from '@/lib/stores/gitStore';

export interface CommitInfo {
  id: string;
  full_hash: string;
  message: string;
  message_body: string;
  author: string;
  author_email: string;
  date: number;
  parent_ids: string[];
  is_pushed: boolean;
  branch_type: string;
  graph_column: number;
}

export interface CommitFileDiff {
  path: string;
  status: string;
  diff: string;
  additions: number;
  deletions: number;
}

export interface CommitDiffResult {
  commit: CommitInfo;
  files: CommitFileDiff[];
  total_additions: number;
  total_deletions: number;
}

export interface PushResult {
  success: boolean;
  message: string;
}

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

  /**
   * Get commit log for a repository
   */
  getCommitLog: (repoPath: string, maxCount?: number, skip?: number): Promise<CommitInfo[]> =>
    invoke('get_commit_log', {
      repoPath,
      maxCount: maxCount ?? null,
      skip: skip ?? null,
    }),

  /**
   * Get diff details for a specific commit
   */
  getCommitDiff: (repoPath: string, commitHash: string): Promise<CommitDiffResult> =>
    invoke('get_commit_diff', { repoPath, commitHash }),

  /**
   * Push commits to remote
   */
  pushCommits: (repoPath: string, remote?: string, branch?: string): Promise<PushResult> =>
    invoke('push_commits', { repoPath, remote: remote ?? null, branch: branch ?? null }),
};
