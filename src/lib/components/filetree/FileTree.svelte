<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import FileTreeItem from './FileTreeItem.svelte';
  import type { FileEntry } from './types';

  interface Props {
    rootPath?: string;
    onFileSelect?: (path: string) => void;
  }

  let { rootPath = '', onFileSelect }: Props = $props();

  let entries = $state<FileEntry[]>([]);
  let selectedPath = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function loadRootDirectory() {
    loading = true;
    error = null;

    try {
      let path = rootPath;
      if (!path) {
        path = await invoke<string>('get_home_directory');
      }

      entries = await invoke<FileEntry[]>('read_directory', { path });
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
    <div class="loading">Loading...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else if entries.length === 0}
    <div class="empty">Empty directory</div>
  {:else}
    {#each entries as entry (entry.path)}
      <FileTreeItem {entry} {selectedPath} onSelect={handleSelect} />
    {/each}
  {/if}
</div>

<style>
  .file-tree {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .loading,
  .error,
  .empty {
    padding: 12px;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .error {
    color: #f44336;
  }
</style>
