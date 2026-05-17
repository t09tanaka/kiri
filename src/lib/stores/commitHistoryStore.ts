// Backward-compatible facade over `commitHistoryState` (issue #42 phase 1).
//
// `commitHistoryState.svelte.ts` is the canonical $state class. This
// facade forwards every mutation to it and mirrors the snapshot into a
// writable so existing `$commitHistoryStore.foo` consumers and the
// `derived(...)` exports below keep working without churn. Once all
// consumers migrate to read from `commitHistoryState.state` directly,
// this file can be deleted.

import { writable, derived } from 'svelte/store';
import type { CommitInfo, CommitDiffResult } from '@/lib/services/gitService';
import { commitHistoryState, type CommitHistoryStateShape } from './commitHistoryState.svelte';

export type CommitHistoryState = CommitHistoryStateShape;

function snapshot(): CommitHistoryStateShape {
  return { ...commitHistoryState.state };
}

function createCommitHistoryStore() {
  const mirror = writable<CommitHistoryStateShape>(snapshot());
  const refresh = () => mirror.set(snapshot());

  return {
    subscribe: mirror.subscribe,

    open: (projectPath: string) => {
      commitHistoryState.open(projectPath);
      refresh();
    },

    close: () => {
      commitHistoryState.close();
      refresh();
    },

    setCommits: (commits: CommitInfo[], pageSize = 50) => {
      commitHistoryState.setCommits(commits, pageSize);
      refresh();
    },

    appendCommits: (newCommits: CommitInfo[], pageSize = 50) => {
      commitHistoryState.appendCommits(newCommits, pageSize);
      refresh();
    },

    setLoadingMore: (loading: boolean) => {
      commitHistoryState.setLoadingMore(loading);
      refresh();
    },

    selectCommit: (hash: string | null) => {
      commitHistoryState.selectCommit(hash);
      refresh();
    },

    setCommitDiff: (diff: CommitDiffResult | null) => {
      commitHistoryState.setCommitDiff(diff);
      refresh();
    },

    setLoadingLog: (loading: boolean) => {
      commitHistoryState.setLoadingLog(loading);
      refresh();
    },

    setLoadingDiff: (loading: boolean) => {
      commitHistoryState.setLoadingDiff(loading);
      refresh();
    },

    setPushing: (pushing: boolean) => {
      commitHistoryState.setPushing(pushing);
      refresh();
    },

    setError: (error: string | null) => {
      commitHistoryState.setError(error);
      refresh();
    },

    markAllPushed: () => {
      commitHistoryState.markAllPushed();
      refresh();
    },

    setPulling: (pulling: boolean) => {
      commitHistoryState.setPulling(pulling);
      refresh();
    },

    setFetching: (fetching: boolean) => {
      commitHistoryState.setFetching(fetching);
      refresh();
    },

    setBehindCount: (count: number) => {
      commitHistoryState.setBehindCount(count);
      refresh();
    },
  };
}

export const commitHistoryStore = createCommitHistoryStore();

export const isCommitHistoryOpen = derived(commitHistoryStore, ($store) => $store.isOpen);

export const unpushedCount = derived(
  commitHistoryStore,
  ($store) => $store.commits.filter((c) => !c.is_pushed).length
);

export const behindCount = derived(commitHistoryStore, ($store) => $store.behindCount);
