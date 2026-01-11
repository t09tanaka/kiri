<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import FileTreeItem from './FileTreeItem.svelte';
  import type { FileEntry } from './types';
  import { gitStore, gitStatusMap } from '@/lib/stores/gitStore';
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

  // Extract project name from rootPath
  const projectName = $derived(rootPath ? rootPath.split('/').pop() || rootPath : null);

  async function loadRootDirectory() {
    loading = true;
    error = null;

    try {
      let path = rootPath;
      if (!path) {
        path = await invoke<string>('get_home_directory');
      }

      entries = await invoke<FileEntry[]>('read_directory', { path });

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

  onMount(() => {
    loadRootDirectory();
  });

  // Reload when rootPath changes
  $effect(() => {
    if (rootPath !== undefined) {
      loadRootDirectory();
    }
  });
</script>

<div class="file-tree">
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
        {#if entries.length === 0}
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
          {#each entries as entry (entry.path)}
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
    z-index: 1;
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
</style>
