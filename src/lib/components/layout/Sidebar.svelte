<script lang="ts">
  import { FileTree } from '@/lib/components/filetree';
  import GitChanges from '@/lib/components/git/GitChanges.svelte';
  import { appStore } from '@/lib/stores/appStore';
  import { gitStore } from '@/lib/stores/gitStore';

  interface Props {
    width?: number;
    rootPath?: string;
    onFileSelect?: (path: string) => void;
  }

  let { width = 250, rootPath = '', onFileSelect }: Props = $props();

  const sidebarMode = $derived($appStore.sidebarMode);
  const changeCount = $derived($gitStore.repoInfo?.statuses.length ?? 0);

  function toggleChanges() {
    if (sidebarMode === 'changes') {
      appStore.setSidebarMode('explorer');
      gitStore.clearDiffs();
    } else {
      appStore.setSidebarMode('changes');
    }
  }
</script>

<aside class="sidebar" style="width: {width}px">
  <div class="sidebar-header">
    <div class="header-icon">
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
        {#if sidebarMode === 'explorer'}
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
        {:else}
          <circle cx="12" cy="12" r="10"></circle>
          <path d="M12 6v6l4 2"></path>
        {/if}
      </svg>
    </div>
    <span class="title">{sidebarMode === 'explorer' ? 'Explorer' : 'Changes'}</span>
  </div>

  <div class="sidebar-content" class:changes-mode={sidebarMode === 'changes'}>
    {#if sidebarMode === 'explorer'}
      <FileTree {rootPath} {onFileSelect} />
    {:else}
      <GitChanges />
    {/if}
  </div>

  <div class="sidebar-footer">
    <button
      class="changes-button"
      class:active={sidebarMode === 'changes'}
      class:has-changes={changeCount > 0}
      onclick={toggleChanges}
      title="{sidebarMode === 'changes'
        ? 'Back to Explorer'
        : 'View Changes'} ({changeCount} files)"
    >
      {#if sidebarMode === 'changes'}
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
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
        </svg>
        <span>Explorer</span>
      {:else}
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
          <path d="M12 6v6l4 2"></path>
        </svg>
        <span>Changes</span>
        {#if changeCount > 0}
          <span class="badge">{changeCount}</span>
        {/if}
      {/if}
    </button>
  </div>
</aside>

<style>
  .sidebar {
    height: 100%;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    animation: sidebarSlideIn 0.3s ease-out;
  }

  @keyframes sidebarSlideIn {
    from {
      opacity: 0;
      transform: translateX(-8px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  /* Subtle gradient overlay */
  .sidebar::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(125, 211, 252, 0.015) 0%, transparent 40%);
    pointer-events: none;
    z-index: 0;
  }

  /* Right edge glow line */
  .sidebar::after {
    content: '';
    position: absolute;
    top: 0;
    right: 0;
    width: 1px;
    height: 100%;
    background: linear-gradient(
      180deg,
      rgba(125, 211, 252, 0.1) 0%,
      rgba(125, 211, 252, 0.03) 50%,
      transparent 100%
    );
    z-index: 2;
  }

  .sidebar-header {
    position: relative;
    height: var(--tabbar-height, 44px);
    padding: 0 var(--space-4);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    overflow: hidden;
  }

  /* Header subtle shimmer */
  .sidebar-header::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(125, 211, 252, 0.03) 50%,
      transparent 100%
    );
    transform: translateX(-100%);
    animation: headerShimmer 8s ease-in-out infinite;
  }

  @keyframes headerShimmer {
    0%,
    100% {
      transform: translateX(-100%);
    }
    50% {
      transform: translateX(100%);
    }
  }

  .header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-color);
    opacity: 0.7;
    transition: all var(--transition-normal);
  }

  .sidebar-header:hover .header-icon {
    opacity: 1;
    transform: scale(1.1);
  }

  .title {
    flex: 1;
    transition: color var(--transition-fast);
  }

  .sidebar-header:hover .title {
    color: var(--text-secondary);
  }

  .sidebar-content {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .sidebar-content.changes-mode::after {
    display: none;
  }

  /* Bottom fade gradient for overflow hint */
  .sidebar-content::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 40px;
    background: linear-gradient(180deg, transparent, var(--bg-secondary));
    pointer-events: none;
    opacity: 0.8;
    z-index: 1;
  }

  .sidebar-footer {
    position: relative;
    z-index: 10;
    padding: var(--space-2);
    border-top: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .changes-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .changes-button:hover {
    background: var(--bg-glass-hover);
    border-color: var(--border-glow);
    color: var(--text-primary);
    transform: translateY(-1px);
  }

  .changes-button:active {
    transform: translateY(0) scale(0.99);
  }

  .changes-button.has-changes {
    border-color: rgba(251, 191, 36, 0.3);
    background: rgba(251, 191, 36, 0.05);
  }

  .changes-button.has-changes:hover {
    border-color: rgba(251, 191, 36, 0.5);
    background: rgba(251, 191, 36, 0.1);
  }

  .changes-button.active {
    background: var(--accent-subtle);
    border-color: var(--accent-color);
    color: var(--accent-color);
  }

  .changes-button svg {
    flex-shrink: 0;
    transition: transform var(--transition-fast);
  }

  .changes-button:hover svg {
    transform: scale(1.1);
  }

  .badge {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    background: var(--git-modified);
    color: var(--bg-primary);
    font-size: 10px;
    font-weight: 700;
    border-radius: 9px;
    transition: all var(--transition-fast);
  }

  .changes-button.has-changes:not(.active) .badge {
    animation: badgePulse 2s ease-in-out infinite;
  }

  @keyframes badgePulse {
    0%,
    100% {
      box-shadow: 0 0 0 0 rgba(251, 191, 36, 0.4);
    }
    50% {
      box-shadow: 0 0 0 4px rgba(251, 191, 36, 0);
    }
  }

  /* Scrollbar styling */
  .sidebar-content :global(::-webkit-scrollbar) {
    width: 6px;
  }

  .sidebar-content :global(::-webkit-scrollbar-track) {
    background: transparent;
  }

  .sidebar-content :global(::-webkit-scrollbar-thumb) {
    background: rgba(125, 211, 252, 0.1);
    border-radius: 3px;
    transition: background 0.2s ease;
  }

  .sidebar-content:hover :global(::-webkit-scrollbar-thumb) {
    background: rgba(125, 211, 252, 0.2);
  }

  .sidebar-content :global(::-webkit-scrollbar-thumb:hover) {
    background: rgba(125, 211, 252, 0.3);
  }
</style>
