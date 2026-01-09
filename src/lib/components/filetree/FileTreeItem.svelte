<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { FileEntry } from './types';
  import { type GitFileStatus, getStatusIcon, getStatusColor } from '@/lib/stores/gitStore';

  interface Props {
    entry: FileEntry;
    depth?: number;
    selectedPath?: string | null;
    onSelect?: (path: string) => void;
    gitStatusMap?: Map<string, GitFileStatus>;
    repoRoot?: string;
  }

  let {
    entry,
    depth = 0,
    selectedPath = null,
    onSelect,
    gitStatusMap = new Map(),
    repoRoot = '',
  }: Props = $props();

  let expanded = $state(false);
  let children = $state<FileEntry[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  const isSelected = $derived(selectedPath === entry.path);
  const paddingLeft = $derived(12 + depth * 16);

  const gitStatus = $derived(() => {
    if (!repoRoot || !entry.path.startsWith(repoRoot)) return null;
    const relativePath = entry.path.slice(repoRoot.length + 1);
    return gitStatusMap.get(relativePath) ?? null;
  });
  const statusIcon = $derived(gitStatus() ? getStatusIcon(gitStatus()!) : '');
  const statusColor = $derived(gitStatus() ? getStatusColor(gitStatus()!) : '');

  async function toggleExpand() {
    if (!entry.is_dir) return;

    if (expanded) {
      expanded = false;
      return;
    }

    loading = true;
    error = null;

    try {
      children = await invoke<FileEntry[]>('read_directory', { path: entry.path });
      expanded = true;
    } catch (e) {
      error = String(e);
      console.error('Failed to read directory:', e);
    } finally {
      loading = false;
    }
  }

  function handleClick() {
    if (entry.is_dir) {
      toggleExpand();
    } else {
      onSelect?.(entry.path);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleClick();
    }
  }

  function getFileIcon(entry: FileEntry): string {
    if (entry.is_dir) {
      return expanded ? '📂' : '📁';
    }

    const ext = entry.name.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts':
      case 'tsx':
        return '🔷';
      case 'js':
      case 'jsx':
        return '🟨';
      case 'svelte':
        return '🔶';
      case 'json':
        return '📋';
      case 'md':
        return '📝';
      case 'css':
      case 'scss':
        return '🎨';
      case 'html':
        return '🌐';
      case 'rs':
        return '🦀';
      case 'toml':
        return '⚙️';
      default:
        return '📄';
    }
  }
</script>

<div class="tree-item-container">
  <button
    class="tree-item"
    class:selected={isSelected}
    class:hidden-file={entry.is_hidden}
    style="padding-left: {paddingLeft}px"
    onclick={handleClick}
    onkeydown={handleKeyDown}
    title={entry.path}
  >
    <span class="icon">{getFileIcon(entry)}</span>
    <span class="name">{entry.name}</span>
    {#if statusIcon}
      <span class="git-status" style="color: {statusColor}">{statusIcon}</span>
    {/if}
    {#if loading}
      <span class="loading">...</span>
    {/if}
  </button>

  {#if expanded && children.length > 0}
    <div class="children">
      {#each children as child (child.path)}
        <svelte:self
          entry={child}
          depth={depth + 1}
          {selectedPath}
          {onSelect}
          {gitStatusMap}
          {repoRoot}
        />
      {/each}
    </div>
  {/if}

  {#if error}
    <div class="error" style="padding-left: {paddingLeft + 16}px">Failed to load</div>
  {/if}
</div>

<style>
  .tree-item-container {
    width: 100%;
  }

  .tree-item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 24px;
    padding-right: 8px;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tree-item:hover {
    background-color: var(--bg-tertiary);
  }

  .tree-item.selected {
    background-color: var(--accent-color);
    color: white;
  }

  .tree-item.hidden-file {
    opacity: 0.6;
  }

  .icon {
    flex-shrink: 0;
    font-size: 14px;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .loading {
    color: var(--text-secondary);
    font-size: 11px;
  }

  .git-status {
    flex-shrink: 0;
    font-size: 11px;
    font-weight: bold;
    margin-left: auto;
    padding-right: 4px;
  }

  .children {
    width: 100%;
  }

  .error {
    color: #f44336;
    font-size: 11px;
    padding: 4px 8px;
  }
</style>
