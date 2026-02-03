<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fileService } from '@/lib/services/fileService';
  import { watcherService } from '@/lib/services/watcherService';
  import { eventService, type UnlistenFn } from '@/lib/services/eventService';
  import { dragDropService } from '@/lib/services/dragDropService';
  import FileTreeItem from './FileTreeItem.svelte';
  import type { FileEntry } from './types';
  import { gitStore, gitStatusMap } from '@/lib/stores/gitStore';
  import {
    dragDropStore,
    isDragging,
    dropTargetPath,
    draggedPaths,
  } from '@/lib/stores/dragDropStore';
  import { toastStore } from '@/lib/stores/toastStore';
  import { Skeleton } from '@/lib/components/ui';

  interface Props {
    rootPath?: string;
    onFileSelect?: (path: string) => void;
  }

  let { rootPath = '', onFileSelect }: Props = $props();

  let entries = $state<FileEntry[]>([]);
  let selectedPath = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let projectExpanded = $state(true);

  // Watcher state
  let unlistenFs: UnlistenFn | null = null;
  let unlistenGit: UnlistenFn | null = null;
  let currentWatchPath: string | null = null;
  let refreshDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Drag and drop state
  let unlistenDragEnter: UnlistenFn | null = null;
  let unlistenDragDrop: UnlistenFn | null = null;
  let unlistenDragLeave: UnlistenFn | null = null;

  // Extract project name from rootPath
  const projectName = $derived(rootPath ? rootPath.split('/').pop() || rootPath : null);

  // Preview entries shown during drag (before drop)
  const previewEntries = $derived.by(() => {
    // Show preview when dragging and targeting root
    if (!$isDragging || !rootPath) return [];
    const targetIsRoot = !$dropTargetPath || $dropTargetPath === rootPath;
    if (!targetIsRoot) return [];

    return $draggedPaths.map((sourcePath) => {
      const name = sourcePath.split('/').pop() || sourcePath;
      return {
        name,
        path: `${rootPath}/${name}`,
        is_dir: false, // We don't know yet, but it doesn't matter for preview
        is_hidden: name.startsWith('.'),
        is_gitignored: false,
        is_pending: true,
      } satisfies FileEntry;
    });
  });

  // Sort entries: directories first, then alphabetically by name (case-insensitive)
  function sortEntries(items: FileEntry[]): FileEntry[] {
    return [...items].sort((a, b) => {
      // Directories come first
      if (a.is_dir !== b.is_dir) {
        return a.is_dir ? -1 : 1;
      }
      // Then sort alphabetically (case-insensitive)
      return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
    });
  }

  // Combined entries: real entries + preview entries (during drag), sorted
  const displayEntries = $derived.by(() => {
    if (previewEntries.length === 0) return entries;
    // Filter out any entries that have the same path as preview entries (avoid duplicates)
    const previewPaths = new Set(previewEntries.map((e) => e.path));
    const filtered = entries.filter((e) => !previewPaths.has(e.path));
    return sortEntries([...filtered, ...previewEntries]);
  });

  async function loadRootDirectory(showLoading = true) {
    if (showLoading) {
      loading = true;
    }
    error = null;

    try {
      let path = rootPath;
      if (!path) {
        path = await fileService.getHomeDirectory();
      }

      entries = await fileService.readDirectory(path);

      gitStore.refresh(path);
    } catch (e) {
      error = String(e);
      console.error('Failed to load directory:', e);
    } finally {
      loading = false;
    }
  }

  function handleSelect(path: string) {
    selectedPath = path;
    onFileSelect?.(path);
  }

  // Debounced refresh to avoid rapid updates
  function scheduleRefresh() {
    if (refreshDebounceTimer) {
      clearTimeout(refreshDebounceTimer);
    }
    refreshDebounceTimer = setTimeout(() => {
      loadRootDirectory(false); // Don't show loading state on refresh
    }, 100);
  }

  async function setupWatcher(path: string) {
    if (!path || currentWatchPath === path) return;

    // Cleanup previous watcher
    await cleanupWatcher();

    try {
      // Start watching
      await watcherService.startWatching(path);
      currentWatchPath = path;

      // Listen for file system changes
      unlistenFs = await eventService.listen<{ path: string }>('fs-changed', (event) => {
        if (event.payload.path === path) {
          scheduleRefresh();
        }
      });

      // Listen for git status changes
      unlistenGit = await eventService.listen<{ repo_root: string }>(
        'git-status-changed',
        (event) => {
          if (path.startsWith(event.payload.repo_root)) {
            gitStore.refresh(path);
          }
        }
      );
    } catch (err) {
      console.error('Failed to setup watcher:', err);
    }
  }

  async function cleanupWatcher() {
    if (refreshDebounceTimer) {
      clearTimeout(refreshDebounceTimer);
      refreshDebounceTimer = null;
    }

    if (unlistenFs) {
      unlistenFs();
      unlistenFs = null;
    }

    if (unlistenGit) {
      unlistenGit();
      unlistenGit = null;
    }

    if (currentWatchPath) {
      await watcherService.stopWatching(currentWatchPath).catch(() => {});
      currentWatchPath = null;
    }
  }

  // Drag and drop handlers
  interface DragPayload {
    paths: string[];
    position: { x: number; y: number };
  }

  async function handleDrop(paths: string[], targetDir: string) {
    try {
      const result = await dragDropService.copyToDirectory(paths, targetDir);

      // Only show notifications for errors (success is shown via optimistic UI)
      if (!result.success) {
        if (result.copied.length > 0) {
          const successCount = result.copied.length;
          const errorCount = result.errors.length;
          toastStore.warning(
            `${successCount} copied, ${errorCount} failed: ${result.errors[0]?.error || 'Unknown error'}`
          );
        } else {
          const errorMsg = result.errors[0]?.error || 'Unknown error';
          toastStore.error(`Copy failed: ${errorMsg}`);
        }
      }
    } catch (e) {
      toastStore.error(`Copy failed: ${String(e)}`);
    }
  }

  async function setupDragDropListeners() {
    // Listen for drag enter
    unlistenDragEnter = await eventService.listen<DragPayload>('tauri://drag-enter', (event) => {
      dragDropStore.startDrag(event.payload.paths);
    });

    // Listen for drag drop
    unlistenDragDrop = await eventService.listen<DragPayload>(
      'tauri://drag-drop',
      async (event) => {
        const targetDir = $dropTargetPath || rootPath;
        if (targetDir) {
          await handleDrop(event.payload.paths, targetDir);
        }
        dragDropStore.endDrag();
      }
    );

    // Listen for drag leave
    unlistenDragLeave = await eventService.listen('tauri://drag-leave', () => {
      dragDropStore.endDrag();
    });
  }

  function cleanupDragDropListeners() {
    if (unlistenDragEnter) {
      unlistenDragEnter();
      unlistenDragEnter = null;
    }
    if (unlistenDragDrop) {
      unlistenDragDrop();
      unlistenDragDrop = null;
    }
    if (unlistenDragLeave) {
      unlistenDragLeave();
      unlistenDragLeave = null;
    }
    dragDropStore.endDrag();
  }

  onMount(() => {
    loadRootDirectory();
    if (rootPath) {
      setupWatcher(rootPath);
    }
    setupDragDropListeners();
  });

  onDestroy(() => {
    cleanupWatcher();
    cleanupDragDropListeners();
  });

  // Reload when rootPath changes
  $effect(() => {
    if (rootPath !== undefined) {
      loadRootDirectory();
      if (rootPath) {
        setupWatcher(rootPath);
      }
    }
  });
</script>

<div class="file-tree" class:drag-active={$isDragging} data-testid="file-tree">
  {#if loading}
    <div class="loading-skeleton">
      {#each Array(6) as _, i (i)}
        <div class="skeleton-item" style="--i: {i}; padding-left: {(i % 3) * 12 + 12}px">
          <Skeleton width="14px" height="14px" borderRadius="3px" />
          <Skeleton width="{60 + Math.random() * 40}%" height="12px" variant="text" />
        </div>
      {/each}
    </div>
  {:else if error}
    <div class="error">
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
        <circle cx="12" cy="12" r="10"></circle>
        <line x1="12" y1="8" x2="12" y2="12"></line>
        <line x1="12" y1="16" x2="12.01" y2="16"></line>
      </svg>
      <span>{error}</span>
    </div>
  {:else}
    <div class="tree-content">
      {#if projectName}
        <button
          class="project-header"
          class:expanded={projectExpanded}
          onclick={() => (projectExpanded = !projectExpanded)}
          title={rootPath}
        >
          <svg
            class="chevron"
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="9 18 15 12 9 6"></polyline>
          </svg>
          <span class="project-name">{projectName}</span>
        </button>
      {/if}
      {#if projectExpanded}
        {#if displayEntries.length === 0}
          <div class="empty">
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
              />
            </svg>
            <span>Empty directory</span>
          </div>
        {:else}
          {#each displayEntries as entry (entry.path)}
            <FileTreeItem
              {entry}
              {selectedPath}
              onSelect={handleSelect}
              gitStatusMap={$gitStatusMap}
              repoRoot={$gitStore.repoInfo?.root ?? ''}
            />
          {/each}
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .file-tree {
    position: relative;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--border-color) transparent;
  }

  /* Fade mask at bottom for scroll indication */
  .file-tree::after {
    content: '';
    position: sticky;
    bottom: 0;
    left: 0;
    right: 0;
    height: 24px;
    background: linear-gradient(to top, var(--bg-secondary), transparent);
    pointer-events: none;
    opacity: 0.8;
  }

  .file-tree::-webkit-scrollbar {
    width: 6px;
  }

  .file-tree::-webkit-scrollbar-track {
    background: transparent;
  }

  .file-tree::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.1);
    border-radius: 4px;
    border: 2px solid transparent;
    background-clip: content-box;
  }

  .file-tree::-webkit-scrollbar-thumb:hover {
    background: rgba(125, 211, 252, 0.15);
    border: 2px solid transparent;
    background-clip: content-box;
  }

  .file-tree::-webkit-scrollbar-thumb:active {
    background: rgba(125, 211, 252, 0.2);
    border: 2px solid transparent;
    background-clip: content-box;
  }

  .tree-content {
    padding: var(--space-1) 0;
    animation: treeContentFadeIn 0.3s ease-out;
  }

  .project-header {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    padding: 6px var(--space-2);
    margin-bottom: 2px;
    background: transparent;
    border: none;
    border-radius: 0;
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-sans);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
  }

  .project-header:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .project-header:active {
    background: var(--bg-active);
  }

  .project-header .chevron {
    flex-shrink: 0;
    transition: transform var(--transition-fast);
    opacity: 0.6;
  }

  .project-header.expanded .chevron {
    transform: rotate(90deg);
  }

  .project-header:hover .chevron {
    opacity: 1;
  }

  .project-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @keyframes treeContentFadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .loading-skeleton {
    display: flex;
    flex-direction: column;
    padding: var(--space-2) var(--space-3);
    gap: var(--space-1);
  }

  .skeleton-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px var(--space-2);
    animation: skeletonFadeIn 0.4s ease backwards;
    animation-delay: calc(var(--i) * 30ms);
  }

  @keyframes skeletonFadeIn {
    from {
      opacity: 0;
      transform: translateX(-8px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .error {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    margin: var(--space-2);
    background: rgba(255, 69, 58, 0.08);
    border: 1px solid rgba(255, 69, 58, 0.2);
    border-radius: var(--radius-md);
    color: var(--git-deleted);
    font-size: 12px;
    animation: errorShake 0.4s ease;
  }

  .error svg {
    flex-shrink: 0;
  }

  @keyframes errorShake {
    0%,
    100% {
      transform: translateX(0);
    }
    20% {
      transform: translateX(-4px);
    }
    40% {
      transform: translateX(4px);
    }
    60% {
      transform: translateX(-2px);
    }
    80% {
      transform: translateX(2px);
    }
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-6);
    color: var(--text-muted);
    font-size: 12px;
    animation: fadeInUp 0.4s ease;
  }

  @keyframes fadeInUp {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .empty svg {
    opacity: 0.4;
    color: var(--accent-color);
    transition: all var(--transition-normal);
  }

  .empty:hover svg {
    opacity: 0.6;
    transform: scale(1.05);
  }

  /* Tree content relative positioning */
  .tree-content {
    position: relative;
  }

  /* Enhanced scrollbar on hover */
  .file-tree:hover::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.2);
    border: 2px solid transparent;
    background-clip: content-box;
  }

  /* Subtle top mist line */
  .file-tree::before {
    content: '';
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: linear-gradient(90deg, transparent, rgba(125, 211, 252, 0.05), transparent);
    pointer-events: none;
    z-index: 2;
    transition: opacity 0.3s ease;
  }

  .file-tree:hover::before {
    background: linear-gradient(90deg, transparent, rgba(125, 211, 252, 0.08), transparent);
  }

  /* Drag and drop active state */
  .file-tree.drag-active {
    background: rgba(125, 211, 252, 0.03);
  }

  .file-tree.drag-active::before {
    background: linear-gradient(90deg, transparent, rgba(125, 211, 252, 0.15), transparent);
  }
</style>
