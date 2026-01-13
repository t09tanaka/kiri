<script lang="ts">
  import DiffView from './DiffView.svelte';
  import { gitStore } from '@/lib/stores/gitStore';
  import { projectStore, currentProjectPath } from '@/lib/stores/projectStore';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  // Local loading state for this window
  let isLoading = $state(false);

  // Real-time update listeners
  let unlistenProject: UnlistenFn | null = null;
  let unlistenGitStatus: UnlistenFn | null = null;
  let unlistenFsChanged: UnlistenFn | null = null;
  let refreshDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    // Listen for project path changes from main window
    unlistenProject = await listen<{ path: string }>('project-path-changed', async (event) => {
      if (event.payload.path) {
        projectStore.setCurrentPath(event.payload.path);
        await loadDiffs(event.payload.path);
      }
    });

    // Listen for git status changes (real-time updates)
    unlistenGitStatus = await listen<{ repo_root: string }>('git-status-changed', (event) => {
      const path = $currentProjectPath;
      if (path && path.startsWith(event.payload.repo_root)) {
        scheduleRefresh();
      }
    });

    // Listen for file system changes (real-time updates)
    unlistenFsChanged = await listen<{ path: string }>('fs-changed', (event) => {
      const path = $currentProjectPath;
      if (path && event.payload.path === path) {
        scheduleRefresh();
      }
    });

    // Get initial project path from main window or use current
    const path = $currentProjectPath;
    if (path) {
      await loadDiffs(path);
    }
  });

  onDestroy(() => {
    // Cleanup listeners
    if (refreshDebounceTimer) {
      clearTimeout(refreshDebounceTimer);
    }
    unlistenProject?.();
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
    const path = $currentProjectPath;
    if (path) {
      await loadDiffs(path);
    }
  }
</script>

<div class="diffview-window">
  <div class="window-header">
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
      {#if $currentProjectPath}
        <span class="project-path">{$currentProjectPath.split('/').pop()}</span>
      {/if}
    </div>
    <button class="refresh-btn" onclick={refreshDiffs} disabled={isLoading} title="Refresh diffs">
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
  </div>
  <div class="content">
    <DiffView />
  </div>
</div>

<style>
  .diffview-window {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .window-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
    padding: 0 var(--space-3);
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-color);
    user-select: none;
    -webkit-app-region: drag;
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
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.02em;
  }

  .project-path {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    padding: 2px 6px;
    background: var(--bg-elevated);
    border-radius: var(--radius-sm);
  }

  .refresh-btn {
    -webkit-app-region: no-drag;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .refresh-btn:hover:not(:disabled) {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .refresh-btn:active:not(:disabled) {
    transform: scale(0.95);
  }

  .refresh-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .refresh-btn svg.spinning {
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

  .content {
    flex: 1;
    overflow: hidden;
  }
</style>
