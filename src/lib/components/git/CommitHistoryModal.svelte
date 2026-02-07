<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { commitHistoryStore, unpushedCount, behindCount } from '@/lib/stores/commitHistoryStore';
  import { gitService } from '@/lib/services/gitService';
  import { eventService, type UnlistenFn } from '@/lib/services/eventService';
  import { toastStore } from '@/lib/stores/toastStore';
  import CommitGraph from './CommitGraph.svelte';
  import CommitDetail from './CommitDetail.svelte';
  import { Spinner } from '@/lib/components/ui';
  import type { CommitInfo } from '@/lib/services/gitService';

  interface Props {
    projectPath: string;
    onClose: () => void;
  }

  let { projectPath, onClose }: Props = $props();
  let mounted = $state(false);
  let unreadHashes: Set<string> = $state(new Set());
  let unlistenGitStatus: UnlistenFn | undefined;
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;
  let fetchInterval: ReturnType<typeof setInterval> | undefined;

  const FETCH_INTERVAL_MS = 30_000;

  onMount(async () => {
    mounted = true;
    document.addEventListener('keydown', handleKeyDown, true);
    await loadCommitLog();

    // Initial fetch and behind/ahead check
    await fetchAndCheckRemote();

    // Periodic fetch while modal is open
    fetchInterval = setInterval(() => fetchAndCheckRemote(), FETCH_INTERVAL_MS);

    // Listen for git status changes (real-time updates)
    unlistenGitStatus = await eventService.listen<{ repo_root: string }>(
      'git-status-changed',
      (event) => {
        if (projectPath.startsWith(event.payload.repo_root)) {
          scheduleRefresh();
        }
      }
    );
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
    unlistenGitStatus?.();
    if (refreshTimer) clearTimeout(refreshTimer);
    if (fetchInterval) clearInterval(fetchInterval);
  });

  const PAGE_SIZE = 50;

  async function loadCommitLog() {
    commitHistoryStore.setLoadingLog(true);
    try {
      const commits = await gitService.getCommitLog(projectPath, PAGE_SIZE);
      commitHistoryStore.setCommits(commits, PAGE_SIZE);
      if (commits.length > 0) {
        await handleSelectCommit(commits[0]);
      }
    } catch (error) {
      commitHistoryStore.setError(String(error));
    }
  }

  function scheduleRefresh() {
    if (refreshTimer) clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => refreshCommitLog(), 300);
  }

  async function refreshCommitLog() {
    const previousHash = $commitHistoryStore.selectedCommitHash;
    const oldHashes = new Set($commitHistoryStore.commits.map((c) => c.full_hash));
    try {
      const commits = await gitService.getCommitLog(projectPath, PAGE_SIZE);
      const newHashes = commits.filter((c) => !oldHashes.has(c.full_hash)).map((c) => c.full_hash);
      if (newHashes.length > 0) {
        unreadHashes = new Set([...unreadHashes, ...newHashes]);
      }
      commitHistoryStore.setCommits(commits, PAGE_SIZE);
      // Preserve previously selected commit if it still exists
      const selected = previousHash
        ? commits.find((c) => c.full_hash === previousHash)
        : commits[0];
      if (selected) {
        await handleSelectCommit(selected);
      } else if (commits.length > 0) {
        await handleSelectCommit(commits[0]);
      }
    } catch (error) {
      commitHistoryStore.setError(String(error));
    }
  }

  async function handleLoadMore() {
    if ($commitHistoryStore.isLoadingMore || !$commitHistoryStore.hasMore) return;
    commitHistoryStore.setLoadingMore(true);
    try {
      const skip = $commitHistoryStore.commits.length;
      const newCommits = await gitService.getCommitLog(projectPath, PAGE_SIZE, skip);
      commitHistoryStore.appendCommits(newCommits, PAGE_SIZE);
    } catch (error) {
      commitHistoryStore.setError(String(error));
    }
  }

  async function handleSelectCommit(commit: CommitInfo) {
    if (unreadHashes.has(commit.full_hash)) {
      const next = new Set(unreadHashes); // eslint-disable-line svelte/prefer-svelte-reactivity
      next.delete(commit.full_hash);
      unreadHashes = next;
    }
    commitHistoryStore.selectCommit(commit.full_hash);
    commitHistoryStore.setLoadingDiff(true);
    try {
      const diff = await gitService.getCommitDiff(projectPath, commit.full_hash);
      // Override commit info from diff with the one from log (which has correct is_pushed state)
      diff.commit = { ...diff.commit, is_pushed: commit.is_pushed };
      commitHistoryStore.setCommitDiff(diff);
    } catch (error) {
      commitHistoryStore.setError(String(error));
    }
  }

  async function fetchAndCheckRemote() {
    if ($commitHistoryStore.isFetching) return;
    commitHistoryStore.setFetching(true);
    try {
      await gitService.fetchRemote(projectPath);
      const counts = await gitService.getBehindAheadCount(projectPath);
      commitHistoryStore.setBehindCount(counts.behind);
    } catch {
      // Silently ignore fetch errors (no remote, network issues, etc.)
    } finally {
      commitHistoryStore.setFetching(false);
    }
  }

  async function handlePull() {
    commitHistoryStore.setPulling(true);
    try {
      const result = await gitService.pullCommits(projectPath);
      if (result.success) {
        commitHistoryStore.setBehindCount(0);
        commitHistoryStore.setPulling(false);
        await refreshCommitLog();
      } else {
        toastStore.error(result.message);
        commitHistoryStore.setPulling(false);
      }
    } catch (error) {
      toastStore.error(String(error));
      commitHistoryStore.setPulling(false);
    }
  }

  async function handlePush() {
    commitHistoryStore.setPushing(true);
    try {
      // Fetch before push to ensure we have latest remote state
      await gitService.fetchRemote(projectPath);
      const counts = await gitService.getBehindAheadCount(projectPath);
      if (counts.behind > 0) {
        commitHistoryStore.setBehindCount(counts.behind);
        toastStore.error(`Pull ${counts.behind} commit${counts.behind > 1 ? 's' : ''} first`);
        commitHistoryStore.setPushing(false);
        return;
      }

      const result = await gitService.pushCommits(projectPath);
      if (result.success) {
        commitHistoryStore.markAllPushed();
      } else {
        toastStore.error(result.message);
        commitHistoryStore.setPushing(false);
      }
    } catch (error) {
      toastStore.error(String(error));
      commitHistoryStore.setPushing(false);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }
</script>

<div
  class="commit-history-backdrop"
  class:mounted
  onclick={handleBackdropClick}
  onkeydown={() => {}}
  role="button"
  tabindex="-1"
>
  <div class="commit-history-modal">
    <div class="modal-glow"></div>
    <div class="modal-content">
      <div class="modal-header">
        <div class="header-content">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <line x1="6" y1="3" x2="6" y2="15"></line>
            <circle cx="18" cy="6" r="3"></circle>
            <circle cx="6" cy="18" r="3"></circle>
            <path d="M18 9a9 9 0 0 1-9 9"></path>
          </svg>
          <span class="title">Commit History</span>
        </div>
        <button class="close-btn" onclick={onClose} title="Close (Esc)">
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      {#if $behindCount > 0}
        <div class="sync-alert pull-alert">
          <span class="sync-message">
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <line x1="12" y1="5" x2="12" y2="19"></line>
              <polyline points="19 12 12 19 5 12"></polyline>
            </svg>
            {$behindCount} commit{$behindCount > 1 ? 's' : ''} to pull from remote
          </span>
          <button
            class="sync-btn pull-btn"
            onclick={handlePull}
            disabled={$commitHistoryStore.isPulling}
          >
            {#if $commitHistoryStore.isPulling}
              <Spinner size="sm" />
            {:else}
              Pull
            {/if}
          </button>
        </div>
      {/if}

      {#if $unpushedCount > 0 && $behindCount === 0}
        <div class="sync-alert push-alert">
          <span class="sync-message">
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <line x1="12" y1="19" x2="12" y2="5"></line>
              <polyline points="5 12 12 5 19 12"></polyline>
            </svg>
            {$unpushedCount} commit{$unpushedCount > 1 ? 's' : ''} to push to remote
          </span>
          <button
            class="sync-btn push-btn"
            onclick={handlePush}
            disabled={$commitHistoryStore.isPushing}
          >
            {#if $commitHistoryStore.isPushing}
              <Spinner size="sm" />
            {:else}
              Push
            {/if}
          </button>
        </div>
      {/if}

      <div class="modal-body">
        {#if $commitHistoryStore.isLoadingLog && $commitHistoryStore.commits.length === 0}
          <div class="loading">
            <Spinner size="lg" />
            <span class="loading-text">Loading commit history...</span>
          </div>
        {:else if $commitHistoryStore.error}
          <div class="error-state">
            <span class="error-text">{$commitHistoryStore.error}</span>
          </div>
        {:else if $commitHistoryStore.commits.length === 0}
          <div class="empty-state">
            <svg
              width="48"
              height="48"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="4"></circle>
              <line x1="1.05" y1="12" x2="7" y2="12"></line>
              <line x1="17.01" y1="12" x2="22.96" y2="12"></line>
            </svg>
            <span class="empty-title">No commits</span>
            <span class="empty-subtitle">This repository has no commit history yet</span>
          </div>
        {:else}
          <div class="split-container">
            <div class="graph-panel">
              <CommitGraph
                commits={$commitHistoryStore.commits}
                selectedHash={$commitHistoryStore.selectedCommitHash}
                onSelectCommit={handleSelectCommit}
                isLoadingMore={$commitHistoryStore.isLoadingMore}
                hasMore={$commitHistoryStore.hasMore}
                onLoadMore={handleLoadMore}
                {unreadHashes}
              />
            </div>
            <div class="detail-panel">
              <CommitDetail
                diff={$commitHistoryStore.selectedCommitDiff}
                isLoading={$commitHistoryStore.isLoadingDiff}
              />
            </div>
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <span class="footer-item">
          <kbd>&#8593;&#8595;</kbd>
          <span>navigate</span>
        </span>
        <span class="footer-item">
          <kbd>Esc</kbd>
          <span>close</span>
        </span>
      </div>
    </div>
  </div>
</div>

<style>
  .commit-history-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .commit-history-backdrop.mounted {
    opacity: 1;
  }

  .commit-history-modal {
    position: relative;
    width: 90%;
    max-width: 1400px;
    height: 85%;
    max-height: 900px;
    min-height: 400px;
    animation: modalSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: translateY(-20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .modal-glow {
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: calc(var(--radius-xl) + 2px);
    opacity: 0.06;
    filter: blur(5px);
    z-index: -1;
    transition: opacity 0.3s ease;
  }

  .commit-history-modal:hover .modal-glow {
    opacity: 0.1;
  }

  .modal-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-glass);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-xl);
    overflow: hidden;
    box-shadow: var(--shadow-lg);
  }

  /* Top border shine effect */
  .modal-content::before {
    content: '';
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent-color), transparent);
    opacity: 0.6;
    z-index: 1;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid var(--border-color);
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .header-content svg {
    color: var(--accent-color);
    opacity: 0.8;
  }

  .title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .close-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .sync-alert {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-4);
    font-size: 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .sync-message {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
  }

  .pull-alert {
    background: rgba(125, 211, 252, 0.04);
  }

  .pull-alert .sync-message {
    color: var(--accent-color);
  }

  .push-alert {
    background: rgba(252, 211, 77, 0.04);
  }

  .push-alert .sync-message {
    color: var(--accent3-color);
  }

  .sync-btn {
    padding: var(--space-1) var(--space-3);
    font-size: 11px;
    font-weight: 500;
    font-family: var(--font-sans);
    border: 1px solid var(--border-color);
    border-radius: 999px;
    background: transparent;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .sync-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .sync-btn.pull-btn {
    color: var(--accent-color);
    border-color: rgba(125, 211, 252, 0.25);
  }

  .sync-btn.pull-btn:hover:not(:disabled) {
    background: rgba(125, 211, 252, 0.1);
    border-color: rgba(125, 211, 252, 0.4);
  }

  .sync-btn.push-btn {
    color: var(--accent3-color);
    border-color: rgba(252, 211, 77, 0.25);
  }

  .sync-btn.push-btn:hover:not(:disabled) {
    background: rgba(252, 211, 77, 0.1);
    border-color: rgba(252, 211, 77, 0.4);
  }

  .modal-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .split-container {
    display: flex;
    height: 100%;
  }

  .graph-panel {
    width: 280px;
    min-width: 240px;
    flex-shrink: 0;
    border-right: 1px solid var(--border-color);
    overflow: hidden;
  }

  .detail-panel {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--space-4);
    color: var(--text-muted);
  }

  .loading-text {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .error-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: var(--space-6);
  }

  .error-text {
    font-size: 13px;
    color: var(--git-deleted);
    text-align: center;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    height: 100%;
    padding: var(--space-6);
    text-align: center;
  }

  .empty-state svg {
    color: var(--accent-color);
    opacity: 0.3;
  }

  .empty-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .empty-subtitle {
    font-size: 12px;
    color: var(--text-muted);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-5);
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-top: 1px solid var(--border-subtle);
  }

  .footer-item {
    font-size: 11px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .footer-item kbd {
    padding: 2px 6px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
    box-shadow: 0 1px 0 var(--bg-primary);
  }

  .footer-item span {
    margin-left: 2px;
  }
</style>
