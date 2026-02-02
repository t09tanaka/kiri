<script lang="ts">
  import { tabStore, activeTab } from '@/lib/stores/tabStore';
  import { gitStore } from '@/lib/stores/gitStore';
  import { currentProjectPath } from '@/lib/stores/projectStore';
  import { diffViewStore } from '@/lib/stores/diffViewStore';
  import { worktreeViewStore } from '@/lib/stores/worktreeViewStore';
  import { isWorktree, worktreeCount, isSubdirectoryOfRepo } from '@/lib/stores/worktreeStore';
  import { toastStore } from '@/lib/stores/toastStore';
  import { appStore } from '@/lib/stores/appStore';

  interface Props {
    onShowShortcuts?: () => void;
  }

  let { onShowShortcuts }: Props = $props();

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
    if (!$currentProjectPath) {
      console.error('No project path available');
      return;
    }
    diffViewStore.open($currentProjectPath);
  }

  function handleWorktreesClick() {
    if (!$currentProjectPath) {
      console.error('No project path available');
      return;
    }
    if ($isSubdirectoryOfRepo) {
      toastStore.warning('Worktrees can only be managed from the repository root.', 4000);
      return;
    }
    worktreeViewStore.open($currentProjectPath);
  }
</script>

<footer class="status-bar">
  <div class="status-left">
    <button
      class="status-item sidebar-toggle"
      onclick={() => appStore.toggleSidebar()}
      title={$appStore.showSidebar ? 'Hide Explorer (⌘B)' : 'Show Explorer (⌘B)'}
    >
      {#if $appStore.showSidebar}
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
          <line x1="9" y1="3" x2="9" y2="21"></line>
          <polyline points="14 9 11 12 14 15"></polyline>
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
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
          <line x1="9" y1="3" x2="9" y2="21"></line>
          <polyline points="13 9 16 12 13 15"></polyline>
        </svg>
      {/if}
    </button>
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
      {#if $isWorktree}
        <span class="status-item worktree-branch" title="Worktree: {gitInfo.branch}">
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
          <span class="worktree-label">WT</span>
          <span>{gitInfo.branch}</span>
        </span>
      {:else}
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
    {/if}
    {#if !$isWorktree}
      <button
        class="status-item worktrees-btn"
        class:disabled={$isSubdirectoryOfRepo}
        onclick={handleWorktreesClick}
        title={$isSubdirectoryOfRepo
          ? 'Worktrees unavailable (open from repo root)'
          : `Worktrees (${$worktreeCount}) - ⌘G`}
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
          <circle cx="18" cy="18" r="3"></circle>
          <circle cx="6" cy="6" r="3"></circle>
          <path d="M6 21V9a9 9 0 0 0 9 9"></path>
        </svg>
        <span>Worktrees</span>
        {#if $worktreeCount > 0}
          <span class="worktrees-count">{$worktreeCount}</span>
        {/if}
        <span class="shortcut-key">⌘G</span>
      </button>
    {/if}
    {#if gitInfo?.branch}
      <button
        class="status-item git-changes"
        class:has-changes={changeCount > 0}
        onclick={handleChangesClick}
        title="Open Changes ({changeCount} files, +{gitInfo?.additions ?? 0} -{gitInfo?.deletions ??
          0}) - ⌘D"
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
        {#if changeCount > 0}
          <span class="change-summary">
            <span class="file-count">{changeCount} files</span>
            <span class="additions">+{gitInfo?.additions ?? 0}</span>
            <span class="deletions">-{gitInfo?.deletions ?? 0}</span>
          </span>
        {:else}
          <span class="no-changes">No changes</span>
        {/if}
        <span class="shortcut-key">⌘D</span>
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

  .sidebar-toggle {
    background: transparent;
    border: none;
    padding: 4px 6px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--text-muted);
    transition: all var(--transition-fast);
  }

  .sidebar-toggle:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
  }

  .sidebar-toggle:active {
    transform: scale(0.95);
    transition: transform 100ms ease;
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

  .worktrees-btn {
    padding: 3px var(--space-2);
    background: rgba(192, 132, 252, 0.1);
    border: none;
    border-radius: var(--radius-sm);
    color: rgb(192, 132, 252);
    font-weight: 500;
    font-size: 10px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .worktrees-btn:hover {
    background: rgba(192, 132, 252, 0.2);
    transform: translateY(-1px);
  }

  .worktrees-btn:active {
    transform: translateY(0) scale(0.97);
  }

  .worktrees-btn.disabled {
    background: rgba(128, 128, 128, 0.1);
    color: var(--text-muted);
    cursor: not-allowed;
    opacity: 0.6;
  }

  .worktrees-btn.disabled:hover {
    background: rgba(128, 128, 128, 0.15);
    transform: none;
  }

  .worktrees-count {
    font-weight: 700;
  }

  .shortcut-key {
    font-size: 10px;
    padding: 2px 5px;
    background: rgba(255, 255, 255, 0.15);
    border-radius: 3px;
    color: var(--text-secondary);
    font-family:
      system-ui,
      -apple-system,
      sans-serif;
    margin-left: 4px;
    border: 1px solid rgba(255, 255, 255, 0.2);
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

  .worktree-branch {
    padding: 3px var(--space-2);
    background: rgba(251, 191, 36, 0.15);
    border-radius: var(--radius-sm);
    color: var(--git-modified);
    font-weight: 500;
    font-size: 10px;
    transition: all var(--transition-fast);
  }

  .worktree-branch:hover {
    background: rgba(251, 191, 36, 0.25);
    transform: translateY(-1px);
  }

  .worktree-label {
    font-size: 9px;
    font-weight: 700;
    padding: 1px 5px;
    background: rgba(251, 191, 36, 0.35);
    color: var(--git-modified);
    border-radius: 3px;
    letter-spacing: 0.05em;
  }

  .git-branch:hover svg,
  .worktree-branch:hover svg {
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
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-weight: 500;
    font-size: 10px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .git-changes.has-changes {
    background: rgba(251, 191, 36, 0.1);
    color: var(--git-modified);
  }

  .git-changes:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--text-secondary);
    transform: translateY(-1px);
  }

  .git-changes.has-changes:hover {
    background: rgba(251, 191, 36, 0.15);
    color: var(--git-modified);
  }

  .git-changes:active {
    transform: translateY(0) scale(0.98);
  }

  .git-changes:hover svg {
    animation: changesPulse 0.6s ease;
  }

  .no-changes {
    color: var(--text-muted);
    font-size: 10px;
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

  .change-summary {
    display: flex;
    align-items: center;
    gap: 6px;
    pointer-events: none;
  }

  .file-count {
    color: var(--git-modified);
    pointer-events: none;
  }

  .additions {
    color: var(--git-added);
    font-weight: 600;
    pointer-events: none;
  }

  .deletions {
    color: var(--git-deleted);
    font-weight: 600;
    pointer-events: none;
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
