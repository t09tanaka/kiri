<script lang="ts">
  import type { RecentProject } from '@/lib/stores/projectStore';

  interface Props {
    project: RecentProject;
    onSelect: () => void;
    onRemove: () => void;
  }

  let { project, onSelect, onRemove }: Props = $props();

  function formatTimeAgo(timestamp: number): string {
    const now = Date.now();
    const diff = now - timestamp;

    const minutes = Math.floor(diff / (1000 * 60));
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    const weeks = Math.floor(diff / (1000 * 60 * 60 * 24 * 7));

    if (minutes < 1) return 'just now';
    if (minutes < 60) return `${minutes} minute${minutes === 1 ? '' : 's'} ago`;
    if (hours < 24) return `${hours} hour${hours === 1 ? '' : 's'} ago`;
    if (days < 7) return `${days} day${days === 1 ? '' : 's'} ago`;
    return `${weeks} week${weeks === 1 ? '' : 's'} ago`;
  }

  function shortenPath(path: string): string {
    const home = path.match(/^\/Users\/[^/]+/)?.[0];
    if (home) {
      return path.replace(home, '~');
    }
    return path;
  }

  function handleRemoveClick(e: MouseEvent) {
    e.stopPropagation();
    onRemove();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onSelect();
    }
  }
</script>

<div class="project-item" onclick={onSelect} onkeydown={handleKeyDown} role="button" tabindex="0">
  <div class="project-icon">
    {#if project.gitBranch}
      <span class="git-icon">*</span>
    {:else}
      <span class="folder-icon">+</span>
    {/if}
  </div>
  <div class="project-info">
    <div class="project-name">{project.name}</div>
    <div class="project-path">{shortenPath(project.path)}</div>
    <div class="project-meta">
      {#if project.gitBranch}
        <span class="branch-name">{project.gitBranch}</span>
        <span class="separator">-</span>
      {:else}
        <span class="no-git">(not a git repo)</span>
        <span class="separator">-</span>
      {/if}
      <span class="time-ago">{formatTimeAgo(project.lastOpened)}</span>
    </div>
  </div>
  <button class="remove-button" onclick={handleRemoveClick} title="Remove from recent"> x </button>
</div>

<style>
  .project-item {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    width: 100%;
    padding: 12px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    transition:
      background-color 0.15s,
      border-color 0.15s;
  }

  .project-item:hover {
    background-color: var(--bg-secondary);
    border-color: var(--border-color);
  }

  .project-item:hover .remove-button {
    opacity: 1;
  }

  .project-icon {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--bg-tertiary);
    border-radius: 6px;
    flex-shrink: 0;
    font-size: 16px;
  }

  .git-icon {
    color: var(--git-added);
  }

  .folder-icon {
    color: var(--text-secondary);
  }

  .project-info {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .project-name {
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .project-path {
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .project-meta {
    font-size: 11px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .branch-name {
    color: var(--git-added);
  }

  .no-git {
    font-style: italic;
    opacity: 0.7;
  }

  .separator {
    opacity: 0.5;
  }

  .time-ago {
    opacity: 0.8;
  }

  .remove-button {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    opacity: 0;
    transition:
      opacity 0.15s,
      background-color 0.15s;
    flex-shrink: 0;
    font-size: 14px;
  }

  .remove-button:hover {
    background-color: var(--bg-tertiary);
    color: var(--text-primary);
  }
</style>
