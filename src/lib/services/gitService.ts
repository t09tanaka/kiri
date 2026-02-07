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

export interface FetchResult {
  success: boolean;
  message: string;
}

export interface BehindAheadCount {
  behind: number;
  ahead: number;
}

export interface PullResult {
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

  /**
   * Fetch from remote
   */
  fetchRemote: (repoPath: string, remote?: string): Promise<FetchResult> =>
    invoke('fetch_remote', { repoPath, remote: remote ?? null }),

  /**
   * Get behind/ahead count relative to upstream
   */
  getBehindAheadCount: (repoPath: string): Promise<BehindAheadCount> =>
    invoke('get_behind_ahead_count', { repoPath }),

  /**
   * Get count of commits ahead of the default branch (main/master).
   * Returns 0 if on the default branch itself.
   */
  getBranchAheadCount: (repoPath: string): Promise<number> =>
    invoke('get_branch_ahead_count', { repoPath }),

  /**
   * Pull commits from remote
   */
  pullCommits: (repoPath: string, remote?: string, branch?: string): Promise<PullResult> =>
    invoke('pull_commits', { repoPath, remote: remote ?? null, branch: branch ?? null }),
};
