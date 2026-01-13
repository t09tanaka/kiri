<script lang="ts">
  import { gitStore, getStatusIcon, getStatusColor } from '@/lib/stores/gitStore';
  import { onMount } from 'svelte';

  // Use repoInfo.statuses for immediate file list display (no waiting for diffs)
  // Filter out Ignored files - they should not appear in the changes list
  const statuses = $derived(
    ($gitStore.repoInfo?.statuses ?? []).filter((s) => s.status !== 'Ignored')
  );
  const sortedStatuses = $derived([...statuses].sort((a, b) => a.path.localeCompare(b.path)));
  const changeCount = $derived(sortedStatuses.length);
  const currentVisibleFile = $derived($gitStore.currentVisibleFile);

  onMount(() => {
    // Load diffs in background - file list shows immediately from statuses
    gitStore.loadAllDiffs();
  });

  function getFileName(path: string): string {
    return path.split('/').pop() ?? path;
  }

  function getDirectory(path: string): string {
    const parts = path.split('/');
    if (parts.length <= 1) return '';
    return parts.slice(0, -1).join('/');
  }

  function scrollToFile(path: string) {
    const element = document.getElementById(`diff-${path.replace(/[^a-zA-Z0-9]/g, '-')}`);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }
</script>

<div class="git-changes">
  <div class="file-list">
    {#if changeCount > 0}
      {#each sortedStatuses as entry (entry.path)}
        <button
          class="file-item"
          class:active={currentVisibleFile === entry.path}
          onclick={() => scrollToFile(entry.path)}
        >
          <span class="status-icon" style="color: {getStatusColor(entry.status)}">
            {getStatusIcon(entry.status)}
          </span>
          <span class="file-name">{getFileName(entry.path)}</span>
          <span class="file-dir">{getDirectory(entry.path)}</span>
        </button>
      {/each}
    {:else}
      <div class="empty-state">
        <svg
          width="32"
          height="32"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
          <polyline points="22 4 12 14.01 9 11.01"></polyline>
        </svg>
        <span>No changes</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .git-changes {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2);
  }

  .file-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    height: 28px;
    padding: 0 var(--space-3);
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-sans);
    text-align: left;
    cursor: pointer;
    border-radius: var(--radius-sm);
    margin: 1px 0;
    transition: all var(--transition-fast);
  }

  .file-item:hover {
    background: var(--bg-tertiary);
  }

  .file-item.active {
    background: linear-gradient(90deg, rgba(125, 211, 252, 0.08) 0%, transparent 100%);
  }

  .file-item.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 2px;
    background: var(--accent-color);
    border-radius: 1px;
    box-shadow: 0 0 6px rgba(125, 211, 252, 0.4);
  }

  .file-item.active .file-name {
    color: var(--text-primary);
  }

  .file-item.active .status-icon {
    opacity: 1;
  }

  .file-item:hover .file-name {
    color: var(--accent-color);
  }

  .file-item:active {
    transform: scale(0.995);
    transition: transform 80ms ease;
  }

  /* Hover mist effect */
  .file-item::after {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(
      circle at var(--mouse-x, 50%) var(--mouse-y, 50%),
      rgba(125, 211, 252, 0.06) 0%,
      transparent 60%
    );
    opacity: 0;
    transition: opacity var(--transition-fast);
    pointer-events: none;
    border-radius: inherit;
  }

  .file-item:hover::after {
    opacity: 1;
  }

  .status-icon {
    flex-shrink: 0;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.05em;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-xs);
    border: 1px solid currentColor;
    background: transparent;
    opacity: 0.9;
    transition: all var(--transition-fast);
  }

  .file-item:hover .status-icon {
    opacity: 1;
    transform: scale(1.05);
  }

  .file-name {
    flex-shrink: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color var(--transition-fast);
  }

  .file-dir {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: 11px;
    text-align: right;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-7) var(--space-4);
    color: var(--text-muted);
    font-size: 12px;
  }

  .empty-state svg {
    opacity: 0.5;
    color: var(--git-added);
  }
</style>
