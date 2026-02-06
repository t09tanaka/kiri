<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { commitHistoryStore, unpushedCount } from '@/lib/stores/commitHistoryStore';
  import { gitService } from '@/lib/services/gitService';
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

  onMount(async () => {
    mounted = true;
    document.addEventListener('keydown', handleKeyDown, true);
    await loadCommitLog();
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
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

  async function handlePush() {
    commitHistoryStore.setPushing(true);
    try {
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

  function getProjectName(path: string): string {
    return path.split('/').pop() || path;
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
            <circle cx="12" cy="12" r="4"></circle>
            <line x1="1.05" y1="12" x2="7" y2="12"></line>
            <line x1="17.01" y1="12" x2="22.96" y2="12"></line>
          </svg>
          <span class="title">Commit History</span>
          <span class="project-path">{getProjectName(projectPath)}</span>
          {#if $commitHistoryStore.commits.length > 0 && $commitHistoryStore.commits[0]?.branch_type}
            <span class="branch-badge">
              <svg
                width="10"
                height="10"
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
            </span>
          {/if}
        </div>
        <div class="header-actions">
          {#if $unpushedCount > 0}
            <button
              class="action-btn push-btn"
              onclick={handlePush}
              disabled={$commitHistoryStore.isPushing}
              title="Push {$unpushedCount} commit{$unpushedCount > 1 ? 's' : ''}"
            >
              {#if $commitHistoryStore.isPushing}
                <Spinner size="sm" />
              {:else}
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
              {/if}
              <span>Push</span>
              <span class="push-count">{$unpushedCount}</span>
            </button>
          {/if}
          <button class="action-btn close-btn" onclick={onClose} title="Close (Esc)">
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
      </div>

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

  .project-path {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    padding: 2px 6px;
    background: var(--bg-elevated);
    border-radius: var(--radius-sm);
    margin-left: var(--space-2);
  }

  .branch-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--git-added);
    padding: 2px 6px;
    background: rgba(74, 222, 128, 0.1);
    border-radius: var(--radius-sm);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 28px;
    padding: 0 var(--space-2);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
    font-size: 12px;
  }

  .action-btn:hover:not(:disabled) {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
  }

  .action-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .action-btn.close-btn {
    width: 28px;
    padding: 0;
  }

  .action-btn.close-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .push-btn {
    background: rgba(252, 211, 77, 0.1);
    color: var(--accent3-color);
    font-weight: 500;
  }

  .push-btn:hover:not(:disabled) {
    background: rgba(252, 211, 77, 0.2);
    color: var(--accent3-color);
  }

  .push-count {
    font-size: 10px;
    font-weight: 700;
    padding: 1px 5px;
    background: rgba(252, 211, 77, 0.25);
    border-radius: 3px;
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
