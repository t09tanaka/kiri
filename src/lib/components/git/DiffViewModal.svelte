<script lang="ts">
  import DiffView from './DiffView.svelte';
  import { gitStore } from '@/lib/stores/gitStore';
  import { onMount, onDestroy } from 'svelte';
  import { eventService, type UnlistenFn } from '@/lib/services/eventService';
  import { Spinner } from '@/lib/components/ui';

  interface Props {
    projectPath: string;
    onClose: () => void;
  }

  let { projectPath, onClose }: Props = $props();

  // Local loading state for this modal
  let isLoading = $state(false);
  let mounted = $state(false);

  // Real-time update listeners
  let unlistenGitStatus: UnlistenFn | null = null;
  let unlistenFsChanged: UnlistenFn | null = null;
  let refreshDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    mounted = true;
    await loadDiffs(projectPath);

    // Use capture phase to intercept before terminal handles it
    document.addEventListener('keydown', handleKeyDown, true);

    // Listen for git status changes (real-time updates)
    unlistenGitStatus = await eventService.listen<{ repo_root: string }>(
      'git-status-changed',
      (event) => {
        if (projectPath && projectPath.startsWith(event.payload.repo_root)) {
          scheduleRefresh();
        }
      }
    );

    // Listen for file system changes (real-time updates)
    unlistenFsChanged = await eventService.listen<{ path: string }>('fs-changed', (event) => {
      if (projectPath && event.payload.path === projectPath) {
        scheduleRefresh();
      }
    });
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
    // Cleanup listeners
    if (refreshDebounceTimer) {
      clearTimeout(refreshDebounceTimer);
    }
    unlistenGitStatus?.();
    unlistenFsChanged?.();
  });

  // Debounced refresh to avoid rapid updates
  function scheduleRefresh() {
    if (refreshDebounceTimer) {
      clearTimeout(refreshDebounceTimer);
    }
    refreshDebounceTimer = setTimeout(() => {
      refreshDiffs();
    }, 300);
  }

  async function loadDiffs(path: string) {
    try {
      isLoading = true;
      // First refresh git status to get repo info
      await gitStore.refresh(path);
      // Then load all diffs
      await gitStore.loadAllDiffs();
    } catch (error) {
      console.error('Failed to load diffs:', error);
    } finally {
      isLoading = false;
    }
  }

  async function refreshDiffs() {
    if (projectPath) {
      await loadDiffs(projectPath);
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
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function getProjectName(path: string): string {
    return path.split('/').pop() || path;
  }
</script>

<div
  class="diffview-backdrop"
  class:mounted
  onclick={handleBackdropClick}
  onkeydown={() => {}}
  role="button"
  tabindex="-1"
>
  <div class="diffview-modal">
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
            <circle cx="12" cy="12" r="10"></circle>
            <path d="M12 6v6l4 2"></path>
          </svg>
          <span class="title">Changes</span>
          <span class="project-path">{getProjectName(projectPath)}</span>
        </div>
        <div class="header-actions">
          <button
            class="action-btn refresh-btn"
            onclick={refreshDiffs}
            disabled={isLoading}
            title="Refresh diffs"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class:spinning={isLoading}
            >
              <polyline points="23 4 23 10 17 10"></polyline>
              <polyline points="1 20 1 14 7 14"></polyline>
              <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
            </svg>
          </button>
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
        {#if isLoading && $gitStore.allDiffs.length === 0}
          <div class="loading">
            <Spinner size="lg" />
            <span class="loading-text">Loading diffs...</span>
          </div>
        {:else}
          <DiffView />
        {/if}
      </div>

      <div class="modal-footer">
        <span class="footer-item">
          <kbd>Esc</kbd>
          <span>close</span>
        </span>
      </div>
    </div>
  </div>
</div>

<style>
  .diffview-backdrop {
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

  .diffview-backdrop.mounted {
    opacity: 1;
  }

  .diffview-modal {
    position: relative;
    width: 90%;
    max-width: 1200px;
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

  .diffview-modal:hover .modal-glow {
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
    background: linear-gradient(90deg, transparent, var(--git-modified), transparent);
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
    color: var(--git-modified);
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

  .action-btn:hover:not(:disabled) {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
  }

  .action-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .action-btn.close-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .action-btn svg.spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .modal-body {
    flex: 1;
    min-height: 0;
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
