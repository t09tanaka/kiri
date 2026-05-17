// Canonical $state class for commit history (issue #42 phase 1).
//
// `commitHistoryStore.ts` is the legacy facade — it forwards every
// mutation here and mirrors the snapshot into a writable so existing
// `$commitHistoryStore.foo` consumers and `derived(...)` exports keep
// working without churn. New code should import `commitHistoryState`
// from this file directly; once all consumers are migrated, the facade
// can be deleted.

import type { CommitInfo, CommitDiffResult } from '@/lib/services/gitService';

export interface CommitHistoryStateShape {
  isOpen: boolean;
  projectPath: string | null;
  commits: CommitInfo[];
  selectedCommitHash: string | null;
  selectedCommitDiff: CommitDiffResult | null;
  isLoadingLog: boolean;
  isLoadingMore: boolean;
  isLoadingDiff: boolean;
  isPushing: boolean;
  isPulling: boolean;
  isFetching: boolean;
  behindCount: number;
  hasMore: boolean;
  error: string | null;
}

function initial(): CommitHistoryStateShape {
  return {
    isOpen: false,
    projectPath: null,
    commits: [],
    selectedCommitHash: null,
    selectedCommitDiff: null,
    isLoadingLog: false,
    isLoadingMore: false,
    isLoadingDiff: false,
    isPushing: false,
    isPulling: false,
    isFetching: false,
    behindCount: 0,
    hasMore: true,
    error: null,
  };
}

class CommitHistoryState {
  state = $state<CommitHistoryStateShape>(initial());

  open(projectPath: string): void {
    this.state = { ...this.state, isOpen: true, projectPath, error: null };
  }

  close(): void {
    this.state = initial();
  }

  setCommits(commits: CommitInfo[], pageSize = 50): void {
    this.state = {
      ...this.state,
      commits,
      isLoadingLog: false,
      hasMore: commits.length >= pageSize,
    };
  }

  appendCommits(newCommits: CommitInfo[], pageSize = 50): void {
    this.state = {
      ...this.state,
      commits: [...this.state.commits, ...newCommits],
      isLoadingMore: false,
      hasMore: newCommits.length >= pageSize,
    };
  }

  setLoadingMore(loading: boolean): void {
    this.state = { ...this.state, isLoadingMore: loading };
  }

  selectCommit(hash: string | null): void {
    this.state = { ...this.state, selectedCommitHash: hash, selectedCommitDiff: null };
  }

  setCommitDiff(diff: CommitDiffResult | null): void {
    this.state = { ...this.state, selectedCommitDiff: diff, isLoadingDiff: false };
  }

  setLoadingLog(loading: boolean): void {
    this.state = { ...this.state, isLoadingLog: loading };
  }

  setLoadingDiff(loading: boolean): void {
    this.state = { ...this.state, isLoadingDiff: loading };
  }

  setPushing(pushing: boolean): void {
    this.state = { ...this.state, isPushing: pushing };
  }

  setError(error: string | null): void {
    this.state = {
      ...this.state,
      error,
      isLoadingLog: false,
      isLoadingMore: false,
      isLoadingDiff: false,
      isPushing: false,
    };
  }

  markAllPushed(): void {
    this.state = {
      ...this.state,
      commits: this.state.commits.map((c) => ({ ...c, is_pushed: true })),
      isPushing: false,
    };
  }

  setPulling(pulling: boolean): void {
    this.state = { ...this.state, isPulling: pulling };
  }

  setFetching(fetching: boolean): void {
    this.state = { ...this.state, isFetching: fetching };
  }

  setBehindCount(count: number): void {
    this.state = { ...this.state, behindCount: count };
  }
}

export const commitHistoryState = new CommitHistoryState();
