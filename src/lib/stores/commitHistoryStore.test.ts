import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  commitHistoryStore,
  isCommitHistoryOpen,
  unpushedCount,
  behindCount,
} from './commitHistoryStore';
import type { CommitInfo, CommitDiffResult } from '@/lib/services/gitService';

function createMockCommit(overrides: Partial<CommitInfo> = {}): CommitInfo {
  return {
    id: 'abc1234',
    full_hash: 'abc1234567890def',
    message: 'feat: add feature',
    message_body: '',
    author: 'Test Author',
    author_email: 'test@example.com',
    date: 1700000000,
    parent_ids: ['parent1'],
    is_pushed: true,
    branch_type: 'local',
    graph_column: 0,
    ...overrides,
  };
}

function createMockCommitDiffResult(overrides: Partial<CommitDiffResult> = {}): CommitDiffResult {
  return {
    commit: createMockCommit(),
    files: [
      {
        path: 'src/main.ts',
        status: 'modified',
        diff: '@@ -1,3 +1,4 @@\n+new line',
        additions: 1,
        deletions: 0,
      },
    ],
    total_additions: 1,
    total_deletions: 0,
    ...overrides,
  };
}

describe('commitHistoryStore', () => {
  beforeEach(() => {
    commitHistoryStore.close();
  });

  describe('initial state', () => {
    it('should start with isOpen as false', () => {
      const state = get(commitHistoryStore);
      expect(state.isOpen).toBe(false);
    });

    it('should start with empty commits', () => {
      const state = get(commitHistoryStore);
      expect(state.commits).toEqual([]);
    });

    it('should start with null projectPath', () => {
      const state = get(commitHistoryStore);
      expect(state.projectPath).toBe(null);
    });

    it('should start with null selectedCommitHash', () => {
      const state = get(commitHistoryStore);
      expect(state.selectedCommitHash).toBe(null);
    });

    it('should start with null selectedCommitDiff', () => {
      const state = get(commitHistoryStore);
      expect(state.selectedCommitDiff).toBe(null);
    });

    it('should start with all loading states as false', () => {
      const state = get(commitHistoryStore);
      expect(state.isLoadingLog).toBe(false);
      expect(state.isLoadingMore).toBe(false);
      expect(state.isLoadingDiff).toBe(false);
      expect(state.isPushing).toBe(false);
      expect(state.isPulling).toBe(false);
      expect(state.isFetching).toBe(false);
    });

    it('should start with behindCount as 0', () => {
      const state = get(commitHistoryStore);
      expect(state.behindCount).toBe(0);
    });

    it('should start with hasMore as true', () => {
      const state = get(commitHistoryStore);
      expect(state.hasMore).toBe(true);
    });

    it('should start with null error', () => {
      const state = get(commitHistoryStore);
      expect(state.error).toBe(null);
    });
  });

  describe('open', () => {
    it('should set isOpen to true and projectPath', () => {
      commitHistoryStore.open('/path/to/project');
      const state = get(commitHistoryStore);
      expect(state.isOpen).toBe(true);
      expect(state.projectPath).toBe('/path/to/project');
    });

    it('should clear error when opening', () => {
      commitHistoryStore.setError('previous error');
      commitHistoryStore.open('/path/to/project');
      const state = get(commitHistoryStore);
      expect(state.error).toBe(null);
    });

    it('should preserve existing commits when reopening', () => {
      const commits = [createMockCommit()];
      commitHistoryStore.setCommits(commits);
      commitHistoryStore.open('/path/to/project');
      const state = get(commitHistoryStore);
      expect(state.commits).toEqual(commits);
    });
  });

  describe('close', () => {
    it('should reset to initial state', () => {
      commitHistoryStore.open('/path/to/project');
      commitHistoryStore.setCommits([createMockCommit()]);
      commitHistoryStore.selectCommit('abc1234');
      commitHistoryStore.close();

      const state = get(commitHistoryStore);
      expect(state.isOpen).toBe(false);
      expect(state.projectPath).toBe(null);
      expect(state.commits).toEqual([]);
      expect(state.selectedCommitHash).toBe(null);
      expect(state.selectedCommitDiff).toBe(null);
      expect(state.isLoadingLog).toBe(false);
      expect(state.isLoadingMore).toBe(false);
      expect(state.isLoadingDiff).toBe(false);
      expect(state.isPushing).toBe(false);
      expect(state.isPulling).toBe(false);
      expect(state.isFetching).toBe(false);
      expect(state.behindCount).toBe(0);
      expect(state.hasMore).toBe(true);
      expect(state.error).toBe(null);
    });
  });

  describe('setCommits', () => {
    it('should update commits array', () => {
      const commits = [createMockCommit({ id: 'a1' }), createMockCommit({ id: 'b2' })];
      commitHistoryStore.setCommits(commits);
      const state = get(commitHistoryStore);
      expect(state.commits).toEqual(commits);
      expect(state.commits).toHaveLength(2);
    });

    it('should clear isLoadingLog', () => {
      commitHistoryStore.setLoadingLog(true);
      commitHistoryStore.setCommits([]);
      const state = get(commitHistoryStore);
      expect(state.isLoadingLog).toBe(false);
    });

    it('should handle empty array', () => {
      commitHistoryStore.setCommits([]);
      const state = get(commitHistoryStore);
      expect(state.commits).toEqual([]);
    });

    it('should set hasMore to true when commits.length >= pageSize', () => {
      const commits = Array.from({ length: 50 }, (_, i) => createMockCommit({ id: `commit-${i}` }));
      commitHistoryStore.setCommits(commits, 50);
      expect(get(commitHistoryStore).hasMore).toBe(true);
    });

    it('should set hasMore to false when commits.length < pageSize', () => {
      const commits = [createMockCommit({ id: 'a1' }), createMockCommit({ id: 'b2' })];
      commitHistoryStore.setCommits(commits, 50);
      expect(get(commitHistoryStore).hasMore).toBe(false);
    });
  });

  describe('appendCommits', () => {
    it('should append new commits to existing list', () => {
      const initial = [createMockCommit({ id: 'a1' })];
      commitHistoryStore.setCommits(initial);
      const more = [createMockCommit({ id: 'b2' }), createMockCommit({ id: 'c3' })];
      commitHistoryStore.appendCommits(more);
      const state = get(commitHistoryStore);
      expect(state.commits).toHaveLength(3);
      expect(state.commits[0].id).toBe('a1');
      expect(state.commits[1].id).toBe('b2');
      expect(state.commits[2].id).toBe('c3');
    });

    it('should clear isLoadingMore', () => {
      commitHistoryStore.setLoadingMore(true);
      commitHistoryStore.appendCommits([]);
      expect(get(commitHistoryStore).isLoadingMore).toBe(false);
    });

    it('should set hasMore to false when fewer than pageSize', () => {
      commitHistoryStore.appendCommits([createMockCommit()], 50);
      expect(get(commitHistoryStore).hasMore).toBe(false);
    });

    it('should set hasMore to true when equal to pageSize', () => {
      const commits = Array.from({ length: 50 }, (_, i) => createMockCommit({ id: `commit-${i}` }));
      commitHistoryStore.appendCommits(commits, 50);
      expect(get(commitHistoryStore).hasMore).toBe(true);
    });

    it('should handle appending empty array', () => {
      const initial = [createMockCommit({ id: 'a1' })];
      commitHistoryStore.setCommits(initial);
      commitHistoryStore.appendCommits([]);
      expect(get(commitHistoryStore).commits).toHaveLength(1);
      expect(get(commitHistoryStore).hasMore).toBe(false);
    });
  });

  describe('setLoadingMore', () => {
    it('should set isLoadingMore to true', () => {
      commitHistoryStore.setLoadingMore(true);
      expect(get(commitHistoryStore).isLoadingMore).toBe(true);
    });

    it('should set isLoadingMore to false', () => {
      commitHistoryStore.setLoadingMore(true);
      commitHistoryStore.setLoadingMore(false);
      expect(get(commitHistoryStore).isLoadingMore).toBe(false);
    });
  });

  describe('selectCommit', () => {
    it('should set selectedCommitHash', () => {
      commitHistoryStore.selectCommit('abc1234');
      const state = get(commitHistoryStore);
      expect(state.selectedCommitHash).toBe('abc1234');
    });

    it('should clear selectedCommitDiff when selecting new commit', () => {
      commitHistoryStore.setCommitDiff(createMockCommitDiffResult());
      commitHistoryStore.selectCommit('new-hash');
      const state = get(commitHistoryStore);
      expect(state.selectedCommitDiff).toBe(null);
    });

    it('should accept null to deselect', () => {
      commitHistoryStore.selectCommit('abc1234');
      commitHistoryStore.selectCommit(null);
      const state = get(commitHistoryStore);
      expect(state.selectedCommitHash).toBe(null);
    });
  });

  describe('setCommitDiff', () => {
    it('should set diff result', () => {
      const diff = createMockCommitDiffResult();
      commitHistoryStore.setCommitDiff(diff);
      const state = get(commitHistoryStore);
      expect(state.selectedCommitDiff).toEqual(diff);
    });

    it('should clear isLoadingDiff', () => {
      commitHistoryStore.setLoadingDiff(true);
      commitHistoryStore.setCommitDiff(createMockCommitDiffResult());
      const state = get(commitHistoryStore);
      expect(state.isLoadingDiff).toBe(false);
    });

    it('should accept null to clear diff', () => {
      commitHistoryStore.setCommitDiff(createMockCommitDiffResult());
      commitHistoryStore.setCommitDiff(null);
      const state = get(commitHistoryStore);
      expect(state.selectedCommitDiff).toBe(null);
    });
  });

  describe('setLoadingLog', () => {
    it('should set isLoadingLog to true', () => {
      commitHistoryStore.setLoadingLog(true);
      expect(get(commitHistoryStore).isLoadingLog).toBe(true);
    });

    it('should set isLoadingLog to false', () => {
      commitHistoryStore.setLoadingLog(true);
      commitHistoryStore.setLoadingLog(false);
      expect(get(commitHistoryStore).isLoadingLog).toBe(false);
    });
  });

  describe('setLoadingDiff', () => {
    it('should set isLoadingDiff to true', () => {
      commitHistoryStore.setLoadingDiff(true);
      expect(get(commitHistoryStore).isLoadingDiff).toBe(true);
    });

    it('should set isLoadingDiff to false', () => {
      commitHistoryStore.setLoadingDiff(true);
      commitHistoryStore.setLoadingDiff(false);
      expect(get(commitHistoryStore).isLoadingDiff).toBe(false);
    });
  });

  describe('setPushing', () => {
    it('should set isPushing to true', () => {
      commitHistoryStore.setPushing(true);
      expect(get(commitHistoryStore).isPushing).toBe(true);
    });

    it('should set isPushing to false', () => {
      commitHistoryStore.setPushing(true);
      commitHistoryStore.setPushing(false);
      expect(get(commitHistoryStore).isPushing).toBe(false);
    });
  });

  describe('setError', () => {
    it('should set error message', () => {
      commitHistoryStore.setError('Something went wrong');
      const state = get(commitHistoryStore);
      expect(state.error).toBe('Something went wrong');
    });

    it('should clear all loading states when setting error', () => {
      commitHistoryStore.setLoadingLog(true);
      commitHistoryStore.setLoadingMore(true);
      commitHistoryStore.setLoadingDiff(true);
      commitHistoryStore.setPushing(true);
      commitHistoryStore.setError('error');
      const state = get(commitHistoryStore);
      expect(state.isLoadingLog).toBe(false);
      expect(state.isLoadingMore).toBe(false);
      expect(state.isLoadingDiff).toBe(false);
      expect(state.isPushing).toBe(false);
    });

    it('should accept null to clear error', () => {
      commitHistoryStore.setError('error');
      commitHistoryStore.setError(null);
      expect(get(commitHistoryStore).error).toBe(null);
    });
  });

  describe('markAllPushed', () => {
    it('should mark all commits as pushed', () => {
      const commits = [
        createMockCommit({ id: 'a1', is_pushed: false }),
        createMockCommit({ id: 'b2', is_pushed: false }),
        createMockCommit({ id: 'c3', is_pushed: true }),
      ];
      commitHistoryStore.setCommits(commits);
      commitHistoryStore.markAllPushed();
      const state = get(commitHistoryStore);
      expect(state.commits.every((c) => c.is_pushed)).toBe(true);
    });

    it('should clear isPushing', () => {
      commitHistoryStore.setPushing(true);
      commitHistoryStore.markAllPushed();
      expect(get(commitHistoryStore).isPushing).toBe(false);
    });

    it('should handle empty commits array', () => {
      commitHistoryStore.markAllPushed();
      expect(get(commitHistoryStore).commits).toEqual([]);
    });
  });

  describe('isCommitHistoryOpen derived store', () => {
    it('should be false initially', () => {
      expect(get(isCommitHistoryOpen)).toBe(false);
    });

    it('should be true when opened', () => {
      commitHistoryStore.open('/project');
      expect(get(isCommitHistoryOpen)).toBe(true);
    });

    it('should be false after closing', () => {
      commitHistoryStore.open('/project');
      commitHistoryStore.close();
      expect(get(isCommitHistoryOpen)).toBe(false);
    });

    it('should track state changes correctly', () => {
      const values: boolean[] = [];
      const unsubscribe = isCommitHistoryOpen.subscribe((value) => {
        values.push(value);
      });

      commitHistoryStore.open('/project');
      commitHistoryStore.close();
      commitHistoryStore.open('/another');

      expect(values).toEqual([false, true, false, true]);

      unsubscribe();
    });
  });

  describe('unpushedCount derived store', () => {
    it('should be 0 with no commits', () => {
      expect(get(unpushedCount)).toBe(0);
    });

    it('should count unpushed commits', () => {
      commitHistoryStore.setCommits([
        createMockCommit({ id: 'a1', is_pushed: false }),
        createMockCommit({ id: 'b2', is_pushed: false }),
        createMockCommit({ id: 'c3', is_pushed: true }),
      ]);
      expect(get(unpushedCount)).toBe(2);
    });

    it('should be 0 when all commits are pushed', () => {
      commitHistoryStore.setCommits([
        createMockCommit({ id: 'a1', is_pushed: true }),
        createMockCommit({ id: 'b2', is_pushed: true }),
      ]);
      expect(get(unpushedCount)).toBe(0);
    });

    it('should update after markAllPushed', () => {
      commitHistoryStore.setCommits([
        createMockCommit({ id: 'a1', is_pushed: false }),
        createMockCommit({ id: 'b2', is_pushed: false }),
      ]);
      expect(get(unpushedCount)).toBe(2);
      commitHistoryStore.markAllPushed();
      expect(get(unpushedCount)).toBe(0);
    });

    it('should reset to 0 after close', () => {
      commitHistoryStore.setCommits([createMockCommit({ id: 'a1', is_pushed: false })]);
      expect(get(unpushedCount)).toBe(1);
      commitHistoryStore.close();
      expect(get(unpushedCount)).toBe(0);
    });
  });

  describe('setPulling', () => {
    it('should set isPulling to true', () => {
      commitHistoryStore.setPulling(true);
      expect(get(commitHistoryStore).isPulling).toBe(true);
    });

    it('should set isPulling to false', () => {
      commitHistoryStore.setPulling(true);
      commitHistoryStore.setPulling(false);
      expect(get(commitHistoryStore).isPulling).toBe(false);
    });
  });

  describe('setFetching', () => {
    it('should set isFetching to true', () => {
      commitHistoryStore.setFetching(true);
      expect(get(commitHistoryStore).isFetching).toBe(true);
    });

    it('should set isFetching to false', () => {
      commitHistoryStore.setFetching(true);
      commitHistoryStore.setFetching(false);
      expect(get(commitHistoryStore).isFetching).toBe(false);
    });
  });

  describe('setBehindCount', () => {
    it('should set behindCount', () => {
      commitHistoryStore.setBehindCount(5);
      expect(get(commitHistoryStore).behindCount).toBe(5);
    });

    it('should set behindCount to 0', () => {
      commitHistoryStore.setBehindCount(5);
      commitHistoryStore.setBehindCount(0);
      expect(get(commitHistoryStore).behindCount).toBe(0);
    });
  });

  describe('behindCount derived store', () => {
    it('should be 0 initially', () => {
      expect(get(behindCount)).toBe(0);
    });

    it('should reflect setBehindCount changes', () => {
      commitHistoryStore.setBehindCount(3);
      expect(get(behindCount)).toBe(3);
    });

    it('should reset to 0 after close', () => {
      commitHistoryStore.setBehindCount(5);
      commitHistoryStore.close();
      expect(get(behindCount)).toBe(0);
    });
  });

  describe('edge cases', () => {
    it('should reset everything when closing after selecting a commit with diff', () => {
      commitHistoryStore.open('/project');
      commitHistoryStore.setCommits([createMockCommit()]);
      commitHistoryStore.selectCommit('abc1234');
      commitHistoryStore.setCommitDiff(createMockCommitDiffResult());
      commitHistoryStore.close();

      const state = get(commitHistoryStore);
      expect(state.isOpen).toBe(false);
      expect(state.projectPath).toBe(null);
      expect(state.commits).toEqual([]);
      expect(state.selectedCommitHash).toBe(null);
      expect(state.selectedCommitDiff).toBe(null);
    });

    it('should notify subscribers on state changes', () => {
      const openStates: boolean[] = [];
      const unsubscribe = commitHistoryStore.subscribe((state) => {
        openStates.push(state.isOpen);
      });

      commitHistoryStore.open('/project');
      commitHistoryStore.close();

      expect(openStates).toEqual([false, true, false]);

      unsubscribe();
    });
  });
});
