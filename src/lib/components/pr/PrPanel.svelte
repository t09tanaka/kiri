<script lang="ts">
  import { onMount } from 'svelte';
  import { Spinner } from '@/lib/components/ui';
  import { prStore } from '@/lib/stores/prStore';
  import { worktreeService } from '@/lib/services/worktreeService';
  import { windowService } from '@/lib/services/windowService';
  import { toastStore } from '@/lib/stores/toastStore';
  import type { PullRequest } from '@/lib/services/prService';
  import { branchToWorktreeName } from '@/lib/utils/gitWorktree';

  interface Props {
    projectPath: string;
    onClose: () => void;
  }

  let { projectPath, onClose }: Props = $props();

  let mounted = $state(false);
  let view = $state<'list' | 'detail'>('list');
  let isOpeningLocally = $state(false);

  onMount(() => {
    requestAnimationFrame(() => {
      mounted = true;
    });
    prStore.refresh(projectPath);
  });

  function handleSelectPr(pr: PullRequest) {
    prStore.selectPr(projectPath, pr.number);
    view = 'detail';
  }

  function handleBack() {
    view = 'list';
    prStore.clearSelection();
  }

  function handleRefresh() {
    prStore.refresh(projectPath);
  }

  /**
   * Compute a simple CI status string for URL param passing.
   */
  function getPrCiStatus(pr: PullRequest): string {
    const checks = pr.status_check_rollup;
    if (!checks || checks.length === 0) return 'unknown';
    const hasFailure = checks.some((c) => c.conclusion === 'FAILURE' || c.conclusion === 'failure');
    if (hasFailure) return 'failure';
    const hasPending = checks.some(
      (c) => c.status === 'IN_PROGRESS' || c.status === 'QUEUED' || c.conclusion === null
    );
    if (hasPending) return 'pending';
    return 'success';
  }

  async function handleOpenLocally() {
    const pr = $prStore.selectedPr;
    if (!pr) return;

    isOpeningLocally = true;
    try {
      const worktreeName = branchToWorktreeName(pr.head_ref_name);
      const worktreeInfo = await worktreeService.create(
        projectPath,
        worktreeName,
        pr.head_ref_name,
        false
      );
      // Open the worktree window with PR metadata so it can show a PR header bar
      await windowService.focusOrCreateWindowWithPr(
        worktreeInfo.path,
        pr.number,
        pr.title,
        pr.head_ref_name,
        getPrCiStatus(pr)
      );
      toastStore.success(`Worktree created for PR #${pr.number}`);
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      toastStore.error(`Failed to create worktree: ${message}`);
    } finally {
      isOpeningLocally = false;
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if (view === 'detail') {
        handleBack();
      } else {
        onClose();
      }
    }
  }

  /**
   * Convert an ISO date string to a relative time string.
   */
  function getRelativeTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = Date.now();
    const diffMs = now - date.getTime();
    const diffSeconds = Math.floor(diffMs / 1000);
    const diffMinutes = Math.floor(diffSeconds / 60);
    const diffHours = Math.floor(diffMinutes / 60);
    const diffDays = Math.floor(diffHours / 24);
    const diffWeeks = Math.floor(diffDays / 7);

    if (diffMinutes < 1) return 'just now';
    if (diffMinutes < 60) return diffMinutes === 1 ? '1m ago' : `${diffMinutes}m ago`;
    if (diffHours < 24) return diffHours === 1 ? '1h ago' : `${diffHours}h ago`;
    if (diffDays < 7) return diffDays === 1 ? '1d ago' : `${diffDays}d ago`;
    if (diffWeeks <= 4) return diffWeeks === 1 ? '1w ago' : `${diffWeeks}w ago`;

    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  /**
   * Get CI status icon and color for a PR.
   */
  function getCiStatusIcon(pr: PullRequest): { icon: string; color: string } {
    const checks = pr.status_check_rollup;
    if (!checks || checks.length === 0) {
      return { icon: '○', color: 'var(--text-muted)' };
    }

    const hasFailure = checks.some((c) => c.conclusion === 'FAILURE' || c.conclusion === 'failure');
    if (hasFailure) {
      return { icon: '✕', color: 'var(--git-deleted)' };
    }

    const hasPending = checks.some(
      (c) => c.status === 'IN_PROGRESS' || c.status === 'QUEUED' || c.conclusion === null
    );
    if (hasPending) {
      return { icon: '◔', color: 'var(--accent3-color)' };
    }

    return { icon: '✓', color: 'var(--git-added)' };
  }

  function getReviewDecisionLabel(decision: string | null): {
    text: string;
    color: string;
  } {
    switch (decision) {
      case 'APPROVED':
        return { text: 'Approved', color: 'var(--git-added)' };
      case 'CHANGES_REQUESTED':
        return { text: 'Changes requested', color: 'var(--git-deleted)' };
      case 'REVIEW_REQUIRED':
        return { text: 'Review required', color: 'var(--accent3-color)' };
      default:
        return { text: 'No reviews', color: 'var(--text-muted)' };
    }
  }

  let prState = $derived($prStore);
</script>

<div
  class="pr-backdrop"
  class:mounted
  role="dialog"
  aria-label="Pull Requests"
  onkeydown={handleKeyDown}
>
  <div class="pr-panel">
    <div class="panel-glow"></div>
    <div class="panel-content">
      <!-- Header -->
      <div class="panel-header">
        <div class="header-content">
          {#if view === 'detail'}
            <button class="action-btn back-btn" onclick={handleBack} title="Back to list">
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                <path
                  d="M10 12L6 8L10 4"
                  stroke="currentColor"
                  stroke-width="1.5"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </button>
          {/if}
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path
              d="M8 1C4.134 1 1 4.134 1 8s3.134 7 7 7 7-3.134 7-7S11.866 1 8 1z"
              stroke="currentColor"
              stroke-width="1.2"
            />
            <path
              d="M5.5 8.5L7 10L10.5 6.5"
              stroke="currentColor"
              stroke-width="1.2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <span class="title">Pull Requests</span>
          {#if view === 'list' && prState.prs.length > 0}
            <span class="pr-count">{prState.prs.length}</span>
          {/if}
        </div>
        <div class="header-actions">
          <button
            class="action-btn refresh-btn"
            onclick={handleRefresh}
            title="Refresh"
            disabled={prState.isLoading}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path
                d="M13.65 2.35A7.95 7.95 0 0 0 8 0a8 8 0 1 0 8 8h-2a6 6 0 1 1-1.76-4.24"
                stroke="currentColor"
                stroke-width="1.2"
                fill="none"
              />
              <path d="M14 0v4h-4" stroke="currentColor" stroke-width="1.2" fill="none" />
            </svg>
          </button>
          <button class="action-btn close-btn" onclick={onClose} title="Close">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path
                d="M4 4L12 12M12 4L4 12"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
            </svg>
          </button>
        </div>
      </div>

      <!-- Body -->
      <div class="panel-body">
        {#if !prState.ghAvailable && !prState.isLoading && !prState.error}
          <!-- gh CLI not available -->
          <div class="setup-guide">
            <div class="setup-icon">
              <svg width="32" height="32" viewBox="0 0 16 16" fill="none">
                <path
                  d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z"
                  fill="currentColor"
                />
              </svg>
            </div>
            <h3 class="setup-title">GitHub CLI required</h3>
            <p class="setup-description">
              Install and authenticate the GitHub CLI to browse pull requests.
            </p>
            <div class="setup-steps">
              <div class="setup-step">
                <span class="step-number">1</span>
                <code>brew install gh</code>
              </div>
              <div class="setup-step">
                <span class="step-number">2</span>
                <code>gh auth login</code>
              </div>
            </div>
          </div>
        {:else if prState.isLoading}
          <!-- Loading -->
          <div class="loading-state">
            <Spinner size="md" />
            <span class="loading-text">Loading pull requests...</span>
          </div>
        {:else if prState.error}
          <!-- Error -->
          <div class="error-state">
            <span class="error-icon">!</span>
            <p class="error-message">{prState.error}</p>
            <button class="btn retry-btn" onclick={handleRefresh}>Retry</button>
          </div>
        {:else if view === 'list'}
          <!-- PR List -->
          {#if prState.prs.length === 0}
            <div class="empty-state">
              <span class="empty-icon">
                <svg width="24" height="24" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M8 1C4.134 1 1 4.134 1 8s3.134 7 7 7 7-3.134 7-7S11.866 1 8 1z"
                    stroke="currentColor"
                    stroke-width="1.2"
                  />
                  <path
                    d="M5.5 8.5L7 10L10.5 6.5"
                    stroke="currentColor"
                    stroke-width="1.2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </span>
              <p class="empty-text">No open pull requests</p>
            </div>
          {:else}
            <div class="pr-list">
              {#each prState.prs as pr (pr.number)}
                {@const ciStatus = getCiStatusIcon(pr)}
                <button class="pr-row" onclick={() => handleSelectPr(pr)}>
                  <span class="pr-number">#{pr.number}</span>
                  <span class="pr-title">{pr.title}</span>
                  <span class="pr-meta">
                    <span class="pr-author">{pr.author_login}</span>
                    <span class="pr-ci" style="color: {ciStatus.color}">{ciStatus.icon}</span>
                    <span class="pr-time">{getRelativeTime(pr.updated_at)}</span>
                  </span>
                </button>
              {/each}
            </div>
          {/if}
        {:else if view === 'detail' && prState.selectedPr}
          <!-- PR Detail -->
          {@const pr = prState.selectedPr}
          {@const ciStatus = getCiStatusIcon(pr)}
          {@const reviewDecision = getReviewDecisionLabel(pr.review_decision)}
          <div class="pr-detail">
            <div class="detail-header">
              <h2 class="detail-title">
                <span class="detail-number">#{pr.number}</span>
                {pr.title}
              </h2>
              <div class="detail-meta">
                <code class="detail-branch">{pr.head_ref_name}</code>
                <span class="detail-ci" style="color: {ciStatus.color}">{ciStatus.icon}</span>
                <span class="detail-author">{pr.author_login}</span>
                <span class="detail-time">{getRelativeTime(pr.updated_at)}</span>
              </div>
              <div class="detail-review" style="color: {reviewDecision.color}">
                {reviewDecision.text}
              </div>
              {#if pr.labels.length > 0}
                <div class="detail-labels">
                  {#each pr.labels as label (label.name)}
                    <span
                      class="label-badge"
                      style="background: #{label.color}33; color: #{label.color}; border-color: #{label.color}55"
                    >
                      {label.name}
                    </span>
                  {/each}
                </div>
              {/if}
            </div>

            {#if pr.body}
              <div class="detail-body">
                <p class="detail-description">{pr.body}</p>
              </div>
            {/if}

            <div class="detail-stats">
              <span class="stat-additions">+{pr.additions}</span>
              <span class="stat-deletions">-{pr.deletions}</span>
              <span class="stat-files">{pr.changed_files} files</span>
            </div>

            {#if pr.files.length > 0}
              <div class="detail-files">
                <h3 class="files-heading">Changed files</h3>
                <div class="files-list">
                  {#each pr.files as file (file.path)}
                    <div class="file-row">
                      <span class="file-path">{file.path}</span>
                      <span class="file-changes">
                        {#if file.additions > 0}
                          <span class="file-additions">+{file.additions}</span>
                        {/if}
                        {#if file.deletions > 0}
                          <span class="file-deletions">-{file.deletions}</span>
                        {/if}
                      </span>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {:else if view === 'detail' && !prState.selectedPr}
          <!-- Loading detail -->
          <div class="loading-state">
            <Spinner size="md" />
            <span class="loading-text">Loading PR details...</span>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="panel-footer">
        {#if view === 'detail'}
          <button
            class="footer-open-locally"
            onclick={handleOpenLocally}
            disabled={isOpeningLocally || !prState.selectedPr}
          >
            {#if isOpeningLocally}
              <Spinner size="sm" />
              <span>Creating...</span>
            {:else}
              <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
                <path
                  d="M2 8h12M8 2v12"
                  stroke="currentColor"
                  stroke-width="1.5"
                  stroke-linecap="round"
                />
              </svg>
              <span>Open locally</span>
            {/if}
          </button>
          <span class="footer-item">
            <kbd>Esc</kbd>
            <span>back</span>
          </span>
        {:else}
          <span class="footer-item">
            <kbd>↵</kbd>
            <span>select</span>
          </span>
          <span class="footer-item">
            <kbd>Esc</kbd>
            <span>close</span>
          </span>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .pr-backdrop {
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

  .pr-backdrop.mounted {
    opacity: 1;
  }

  .pr-panel {
    position: relative;
    width: min(520px, 90vw);
    max-height: 80vh;
    animation: panelSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes panelSlideIn {
    from {
      opacity: 0;
      transform: translateY(-20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .panel-glow {
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: calc(var(--radius-xl) + 2px);
    opacity: 0.06;
    filter: blur(5px);
    z-index: -1;
    transition: opacity 0.3s ease;
  }

  .pr-panel:hover .panel-glow {
    opacity: 0.1;
  }

  .panel-content {
    display: flex;
    flex-direction: column;
    background: var(--bg-glass);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-xl);
    overflow: hidden;
    box-shadow: var(--shadow-lg);
  }

  /* Top border shine effect */
  .panel-content::before {
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

  /* Header */
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid var(--border-color);
    border-radius: var(--radius-xl) var(--radius-xl) 0 0;
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
    font-weight: 500;
    color: var(--text-primary);
  }

  .pr-count {
    font-size: 11px;
    padding: 1px 6px;
    background: rgba(125, 211, 252, 0.15);
    color: var(--accent-color);
    border-radius: var(--radius-full);
    font-weight: 500;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .action-btn {
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

  .action-btn:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--text-secondary);
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .back-btn:hover {
    color: var(--accent-color);
  }

  .close-btn:hover {
    background: rgba(248, 113, 113, 0.15);
    color: var(--git-deleted);
  }

  .refresh-btn:hover svg {
    transform: rotate(90deg);
    transition: transform 0.3s ease;
  }

  /* Body */
  .panel-body {
    position: relative;
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
    min-height: 220px;
    max-height: calc(80vh - 110px);
  }

  /* Setup guide (gh not available) */
  .setup-guide {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-6) var(--space-4);
    text-align: center;
  }

  .setup-icon {
    color: var(--text-muted);
    opacity: 0.6;
  }

  .setup-title {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
  }

  .setup-description {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
    max-width: 280px;
    line-height: 1.5;
  }

  .setup-steps {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: 100%;
    max-width: 260px;
  }

  .setup-step {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
  }

  .step-number {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: var(--radius-full);
    background: rgba(125, 211, 252, 0.15);
    color: var(--accent-color);
    font-size: 11px;
    font-weight: 600;
    flex-shrink: 0;
  }

  .setup-step code {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-secondary);
  }

  /* Loading state */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-8) var(--space-4);
  }

  .loading-text {
    font-size: 12px;
    color: var(--text-muted);
  }

  /* Error state */
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-6) var(--space-4);
    text-align: center;
  }

  .error-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-full);
    background: rgba(248, 113, 113, 0.15);
    color: var(--git-deleted);
    font-size: 16px;
    font-weight: 700;
  }

  .error-message {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
    max-width: 320px;
    line-height: 1.5;
  }

  /* Empty state */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-8) var(--space-4);
  }

  .empty-icon {
    color: var(--text-muted);
    opacity: 0.4;
  }

  .empty-text {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
  }

  /* PR List */
  .pr-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .pr-row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: var(--space-2);
    align-items: center;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
    width: 100%;
    font-family: inherit;
  }

  .pr-row:hover {
    background: rgba(125, 211, 252, 0.08);
  }

  .pr-number {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--accent-color);
    opacity: 0.8;
    white-space: nowrap;
  }

  .pr-title {
    font-size: 13px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pr-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
  }

  .pr-author {
    font-size: 11px;
    color: var(--text-muted);
    max-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pr-ci {
    font-size: 12px;
    line-height: 1;
  }

  .pr-time {
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  /* PR Detail */
  .pr-detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .detail-header {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .detail-title {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
    line-height: 1.4;
  }

  .detail-number {
    color: var(--accent-color);
    opacity: 0.8;
    margin-right: var(--space-1);
  }

  .detail-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .detail-branch {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 2px 6px;
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
    border-radius: var(--radius-sm);
  }

  .detail-ci {
    font-size: 13px;
    line-height: 1;
  }

  .detail-author {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .detail-time {
    font-size: 11px;
    color: var(--text-muted);
  }

  .detail-review {
    font-size: 12px;
    font-weight: 500;
  }

  .detail-labels {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .label-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: var(--radius-full);
    border: 1px solid;
    font-weight: 500;
  }

  /* Detail body */
  .detail-body {
    padding: var(--space-3);
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
  }

  .detail-description {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* Stats */
  .detail-stats {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    font-size: 12px;
    font-family: var(--font-mono);
  }

  .stat-additions {
    color: var(--git-added);
  }

  .stat-deletions {
    color: var(--git-deleted);
  }

  .stat-files {
    color: var(--text-muted);
  }

  /* Changed files */
  .detail-files {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .files-heading {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin: 0;
  }

  .files-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    max-height: 200px;
    overflow-y: auto;
  }

  .file-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    transition: background var(--transition-fast);
  }

  .file-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .file-path {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }

  .file-changes {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-shrink: 0;
    font-size: 11px;
    font-family: var(--font-mono);
  }

  .file-additions {
    color: var(--git-added);
  }

  .file-deletions {
    color: var(--git-deleted);
  }

  /* Footer */
  .panel-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--space-5);
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-top: 1px solid var(--border-subtle);
    border-radius: 0 0 var(--radius-xl) var(--radius-xl);
  }

  .footer-open-locally {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: rgba(125, 211, 252, 0.15);
    border: 1px solid rgba(125, 211, 252, 0.3);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-sans);
    color: var(--accent-color);
    cursor: pointer;
    transition: all var(--transition-fast);
    white-space: nowrap;
    margin-right: auto;
  }

  .footer-open-locally:hover {
    background: rgba(125, 211, 252, 0.25);
  }

  .footer-open-locally:active {
    transform: scale(0.98);
  }

  .footer-open-locally:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
    transition: all var(--transition-fast);
  }

  .footer-item:hover kbd {
    color: var(--accent-color);
    border-color: var(--accent-subtle);
    transform: translateY(-1px);
    box-shadow: 0 2px 0 var(--bg-primary);
  }

  .footer-item span {
    margin-left: 2px;
  }

  .retry-btn {
    margin-top: var(--space-1);
  }
</style>
