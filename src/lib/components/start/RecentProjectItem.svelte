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
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    if (days < 7) return `${days}d ago`;
    return `${weeks}w ago`;
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
  <div class="item-bg"></div>
  <div class="item-content">
    <div class="project-icon">
      {#if project.gitBranch}
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <line x1="6" y1="3" x2="6" y2="15"></line>
          <circle cx="18" cy="6" r="3"></circle>
          <circle cx="6" cy="18" r="3"></circle>
          <path d="M18 9a9 9 0 0 1-9 9"></path>
        </svg>
      {:else}
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
        </svg>
      {/if}
    </div>
    <div class="project-info">
      <div class="project-name">{project.name}</div>
      <div class="project-path">{shortenPath(project.path)}</div>
    </div>
    <div class="project-meta">
      <span class="time-ago">{formatTimeAgo(project.lastOpened)}</span>
    </div>
    <button class="remove-button" onclick={handleRemoveClick} title="Remove from recent">
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
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>
  </div>
</div>

<style>
  .project-item {
    position: relative;
    width: 100%;
    cursor: pointer;
    border-radius: var(--radius-md);
    overflow: hidden;
    transition: transform var(--transition-fast);
    animation: itemSlideIn 0.3s ease-out backwards;
  }

  @keyframes itemSlideIn {
    from {
      opacity: 0;
      transform: translateX(-12px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .project-item:hover {
    transform: translateX(4px);
  }

  .project-item:active {
    transform: translateX(4px) scale(0.99);
  }

  /* Hover indicator line */
  .project-item::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 0;
    background: linear-gradient(180deg, var(--gradient-start), var(--gradient-end));
    border-radius: 0 2px 2px 0;
    transition: all var(--transition-fast);
    z-index: 1;
  }

  .project-item:hover::before {
    height: 24px;
  }

  .item-bg {
    position: absolute;
    inset: 0;
    background: var(--bg-glass);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    transition: all var(--transition-normal);
  }

  .project-item:hover .item-bg {
    background: var(--bg-glass-hover);
    border-color: var(--border-color);
  }

  .item-content {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
  }

  .project-item:hover .remove-button {
    opacity: 1;
  }

  .project-icon {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--accent-subtle) 0%, rgba(196, 181, 253, 0.08) 100%);
    border-radius: var(--radius-md);
    flex-shrink: 0;
    color: var(--text-muted);
    transition: all var(--transition-normal);
  }

  .project-item:hover .project-icon {
    color: var(--accent-color);
    background: var(--accent-muted);
    transform: scale(1.05);
  }

  .project-icon svg {
    transition: transform var(--transition-fast);
  }

  .project-item:hover .project-icon svg {
    transform: translateY(-1px);
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
    color: var(--text-primary);
    transition: color var(--transition-fast);
  }

  .project-item:hover .project-name {
    color: var(--accent-color);
  }

  .project-path {
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-mono);
    transition: all var(--transition-fast);
  }

  .project-item:hover .project-path {
    color: var(--text-secondary);
  }

  .project-meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    flex-shrink: 0;
  }

  .time-ago {
    font-size: 10px;
    color: var(--text-muted);
    transition: all var(--transition-fast);
  }

  .project-item:hover .time-ago {
    color: var(--text-secondary);
  }

  .remove-button {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    opacity: 0;
    transition: all var(--transition-fast);
    flex-shrink: 0;
  }

  .remove-button:hover {
    background: rgba(255, 69, 58, 0.15);
    color: var(--git-deleted);
  }

  .remove-button:hover svg {
    transform: scale(1.1);
  }

  .remove-button svg {
    transition: transform var(--transition-fast);
  }

  .remove-button:active {
    transform: scale(0.9);
    background: rgba(255, 69, 58, 0.25);
  }

  /* Hover ripple effect */
  .project-item::after {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(
      circle at var(--mouse-x, 50%) var(--mouse-y, 50%),
      rgba(125, 211, 252, 0.05) 0%,
      transparent 50%
    );
    opacity: 0;
    transition: opacity var(--transition-fast);
    border-radius: inherit;
    pointer-events: none;
    z-index: 1;
  }

  .project-item:hover::after {
    opacity: 0.8;
  }

  /* Item background glow */
  .item-bg::before {
    content: '';
    position: absolute;
    inset: -1px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: inherit;
    opacity: 0;
    z-index: -1;
    filter: blur(3px);
    transition: opacity var(--transition-fast);
  }

  .project-item:hover .item-bg::before {
    opacity: 0.04;
  }

  .project-icon {
    position: relative;
  }

  /* Focus state */
  .project-item:focus-visible {
    outline: none;
  }

  .project-item:focus-visible .item-bg {
    border-color: var(--accent-color);
  }
</style>
