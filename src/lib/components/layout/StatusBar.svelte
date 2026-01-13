<script lang="ts">
  import { tabStore, activeTab } from '@/lib/stores/tabStore';
  import { gitStore } from '@/lib/stores/gitStore';
  import { appStore, type SidebarMode } from '@/lib/stores/appStore';
  import { get } from 'svelte/store';
  import { tick } from 'svelte';

  interface Props {
    onShowShortcuts?: () => void;
  }

  let { onShowShortcuts }: Props = $props();

  // Use $state and $effect for explicit store subscription in Svelte 5
  let sidebarMode = $state<SidebarMode>(get(appStore).sidebarMode);

  $effect(() => {
    const unsubscribe = appStore.subscribe(async (state) => {
      sidebarMode = state.sidebarMode;
      await tick(); // Force synchronous DOM update
    });
    return unsubscribe;
  });

  function getActiveInfo(): { mode: string; file: string | null } {
    const tab = $activeTab;
    if (!tab) {
      return { mode: 'No Tab', file: null };
    }
    if (tab.type === 'terminal') {
      return { mode: 'Terminal', file: null };
    }
    return { mode: 'Editor', file: tab.filePath };
  }

  const info = $derived(getActiveInfo());
  const gitInfo = $derived($gitStore.repoInfo);
  const changeCount = $derived(
    $gitStore.repoInfo?.statuses.filter((s) => s.status !== 'Ignored').length ?? 0
  );

  function handleChangesClick() {
    const wasChangesMode = get(appStore).sidebarMode === 'changes';
    appStore.toggleSidebarMode();
    if (wasChangesMode) {
      gitStore.clearDiffs();
    }
  }
</script>

<footer class="status-bar">
  <div class="status-left">
    <span class="status-item mode">
      {#if info.mode === 'Terminal'}
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polyline points="4 17 10 11 4 5"></polyline>
          <line x1="12" y1="19" x2="20" y2="19"></line>
        </svg>
      {:else}
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
          <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
        </svg>
      {/if}
      <span>{info.mode}</span>
    </span>
    <span class="status-item tab-count">
      <svg
        width="12"
        height="12"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
        <line x1="3" y1="9" x2="21" y2="9"></line>
      </svg>
      <span>{$tabStore.tabs.length} tabs</span>
    </span>
  </div>
  <div class="status-right">
    {#if gitInfo?.branch}
      <span class="status-item git-branch" title="Git branch: {gitInfo.branch}">
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <line x1="6" y1="3" x2="6" y2="15"></line>
          <circle cx="18" cy="6" r="3"></circle>
          <circle cx="6" cy="18" r="3"></circle>
          <path d="M18 9a9 9 0 0 1-9 9"></path>
        </svg>
        <span>{gitInfo.branch}</span>
      </span>
    {/if}
    {#if changeCount > 0}
      <button
        class="status-item git-changes"
        class:active={sidebarMode === 'changes'}
        onclick={handleChangesClick}
        title="View changes ({changeCount} files)"
      >
        <svg
          width="12"
          height="12"
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
        <span>{changeCount}</span>
      </button>
    {/if}
    {#if info.file}
      <span class="status-item file-path" title={info.file}>
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
          <polyline points="14 2 14 8 20 8"></polyline>
        </svg>
        <span>{info.file.split('/').slice(-2).join('/')}</span>
      </span>
    {/if}
    <button
      class="status-item shortcut-hint"
      onclick={onShowShortcuts}
      title="Keyboard Shortcuts (?)"
    >
      <svg
        width="12"
        height="12"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <rect x="2" y="4" width="20" height="16" rx="2" ry="2"></rect>
        <path d="M6 8h.001"></path>
        <path d="M10 8h.001"></path>
        <path d="M14 8h.001"></path>
        <path d="M18 8h.001"></path>
        <path d="M8 12h.001"></path>
        <path d="M12 12h.001"></path>
        <path d="M16 12h.001"></path>
        <path d="M7 16h10"></path>
      </svg>
    </button>
    <span class="status-item brand">
      <span class="brand-name">kiri</span>
      <span class="brand-kanji">霧</span>
    </span>
  </div>
</footer>

<style>
  .status-bar {
    height: var(--statusbar-height);
    background: var(--bg-tertiary);
    border-top: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--space-4);
    font-size: 11px;
    color: var(--text-secondary);
    user-select: none;
    position: relative;
  }

  /* Subtle top highlight */
  .status-bar::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: linear-gradient(
      90deg,
      transparent 15%,
      rgba(125, 211, 252, 0.06) 50%,
      transparent 85%
    );
    pointer-events: none;
    transition: opacity 0.3s ease;
  }

  .status-bar:hover::before {
    background: linear-gradient(
      90deg,
      transparent 10%,
      rgba(125, 211, 252, 0.1) 50%,
      transparent 90%
    );
  }

  .status-left,
  .status-right {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
    transition: all var(--transition-fast);
  }

  .status-item svg {
    transition: transform var(--transition-fast);
  }

  .status-item:hover svg {
    transform: scale(1.1);
  }

  .mode {
    padding: 3px var(--space-2);
    background: var(--accent-subtle);
    border-radius: var(--radius-sm);
    color: var(--accent-color);
    font-weight: 500;
    font-size: 10px;
    letter-spacing: 0.02em;
    transition: all var(--transition-fast);
  }

  .mode:hover {
    background: var(--accent-muted);
    transform: translateY(-1px);
  }

  .mode:active {
    transform: translateY(0) scale(0.97);
  }

  .tab-count {
    color: var(--text-muted);
    font-size: 10px;
    padding: 3px var(--space-2);
    border-radius: var(--radius-sm);
  }

  .tab-count:hover {
    color: var(--text-secondary);
    background: rgba(125, 211, 252, 0.05);
  }

  .git-branch {
    padding: 3px var(--space-2);
    background: rgba(74, 222, 128, 0.1);
    border-radius: var(--radius-sm);
    color: var(--git-added);
    font-weight: 500;
    font-size: 10px;
    transition: all var(--transition-fast);
  }

  .git-branch:hover {
    background: rgba(74, 222, 128, 0.15);
    transform: translateY(-1px);
  }

  .git-branch:hover svg {
    animation: branchPulse 0.6s ease;
  }

  @keyframes branchPulse {
    0%,
    100% {
      transform: scale(1.1);
    }
    50% {
      transform: scale(1.2);
    }
  }

  .git-changes {
    padding: 3px var(--space-2);
    background: rgba(251, 191, 36, 0.1);
    border: none;
    border-radius: var(--radius-sm);
    color: var(--git-modified);
    font-weight: 500;
    font-size: 10px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .git-changes:hover {
    background: rgba(251, 191, 36, 0.15);
    transform: translateY(-1px);
  }

  .git-changes:active {
    transform: translateY(0) scale(0.98);
  }

  .git-changes.active {
    background: rgba(251, 191, 36, 0.2);
    box-shadow: 0 0 8px rgba(251, 191, 36, 0.2);
  }

  .git-changes:hover svg {
    animation: changesPulse 0.6s ease;
  }

  @keyframes changesPulse {
    0%,
    100% {
      transform: scale(1.1);
    }
    50% {
      transform: scale(1.2) rotate(10deg);
    }
  }

  .file-path {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: 10px;
    font-family: var(--font-mono);
    padding: 3px var(--space-2);
    border-radius: var(--radius-sm);
  }

  .file-path:hover {
    color: var(--text-secondary);
    background: rgba(125, 211, 252, 0.05);
  }

  .shortcut-hint {
    background: transparent;
    border: none;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--text-muted);
    transition: all var(--transition-fast);
  }

  .shortcut-hint:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
  }

  .shortcut-hint:active {
    transform: scale(0.95);
    transition: transform 100ms ease;
  }

  .brand {
    padding-left: var(--space-3);
    border-left: 1px solid var(--border-subtle);
    display: flex;
    align-items: baseline;
    gap: 4px;
    transition: all var(--transition-fast);
  }

  .brand:hover .brand-name {
    color: var(--accent-color);
  }

  .brand:hover .brand-kanji {
    opacity: 0.8;
  }

  .brand-name {
    font-family: var(--font-display);
    font-weight: 500;
    font-size: 11px;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    transition: color var(--transition-fast);
  }

  .brand-kanji {
    font-size: 10px;
    color: var(--accent-color);
    opacity: 0.5;
    transition: all var(--transition-fast);
    text-shadow: 0 0 4px rgba(125, 211, 252, 0.2);
  }

  .brand:hover .brand-kanji {
    opacity: 0.8;
    transform: translateY(-1px);
    text-shadow: 0 0 6px rgba(125, 211, 252, 0.3);
  }
</style>
