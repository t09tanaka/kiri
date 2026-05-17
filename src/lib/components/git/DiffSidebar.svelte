<script lang="ts">
  import { getStatusIcon, getStatusColor } from '@/lib/stores/gitStore';
  import { getFileIconInfo } from '@/lib/utils/fileIcons';
  import { computeDiffStats, getFileName, type DiffStats } from './diffParser';
  import { createDiffCache } from '@/lib/utils/diffCache';

  interface FileDiff {
    path: string;
    status: string;
    diff: string;
  }

  interface Props {
    files: FileDiff[];
    totalAdditions: number;
    totalDeletions: number;
    currentVisibleFile: string | null;
    onSelect: (path: string) => void;
  }

  let { files, totalAdditions, totalDeletions, currentVisibleFile, onSelect }: Props = $props();

  // Diff stats are computed lazily and cached so scrolling through many
  // files doesn't re-parse identical content on every render.
  const statsCache = createDiffCache<DiffStats>();
  $effect(() => {
    void files;
    statsCache.clear();
  });

  function statsFor(file: FileDiff): DiffStats {
    return statsCache.getOrCompute(file.path, () => computeDiffStats(file.diff));
  }
</script>

<div class="file-sidebar">
  <div class="sidebar-header">
    <span class="sidebar-title">FILES</span>
    <span class="file-stats">
      <span class="stat-additions">+{totalAdditions}</span>
      <span class="stat-deletions">-{totalDeletions}</span>
    </span>
  </div>
  <div class="file-list">
    {#each files as fileDiff (fileDiff.path)}
      {@const stats = statsFor(fileDiff)}
      {@const fileIconInfo = getFileIconInfo(getFileName(fileDiff.path))}
      <button
        class="file-item"
        class:selected={currentVisibleFile === fileDiff.path}
        onclick={() => onSelect(fileDiff.path)}
        title={fileDiff.path}
      >
        <span class="file-status" style="color: {getStatusColor(fileDiff.status)}">
          {getStatusIcon(fileDiff.status)}
        </span>
        <span class="file-item-name" style="color: {fileIconInfo.color}"
          >{getFileName(fileDiff.path)}</span
        >
        {#if stats.additions > 0 || stats.deletions > 0}
          <span class="file-item-stats">
            {#if stats.additions > 0}
              <span class="stat-add">+{stats.additions}</span>
            {/if}
            {#if stats.deletions > 0}
              <span class="stat-del">-{stats.deletions}</span>
            {/if}
          </span>
        {/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .file-sidebar {
    width: 200px;
    min-width: 150px;
    max-width: 300px;
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: var(--tabbar-height, 44px);
    padding: 0 var(--space-3);
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .sidebar-title {
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }

  .file-stats {
    display: flex;
    gap: var(--space-2);
    font-size: 10px;
    font-weight: 600;
  }

  .stat-additions {
    color: var(--git-added);
  }

  .stat-deletions {
    color: var(--git-deleted);
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2) 0;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background var(--transition-fast);
  }

  .file-item:hover {
    background: var(--bg-elevated);
  }

  .file-item.selected {
    background: var(--accent-subtle);
  }

  .file-status {
    font-size: 10px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .file-item-name {
    font-size: 12px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .file-item-stats {
    display: flex;
    gap: 4px;
    font-size: 10px;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .file-item-stats .stat-add {
    color: var(--git-added);
  }

  .file-item-stats .stat-del {
    color: var(--git-deleted);
  }
</style>
