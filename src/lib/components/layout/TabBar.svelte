<script lang="ts">
  import { tabStore, type Tab } from '@/lib/stores/tabStore';
  import { confirmDialogStore } from '@/lib/stores/confirmDialogStore';

  interface Props {
    tabs: Tab[];
    activeTabId: string | null;
  }

  let { tabs, activeTabId }: Props = $props();

  function handleTabClick(tab: Tab) {
    tabStore.setActiveTab(tab.id);
  }

  async function handleCloseClick(e: MouseEvent, tab: Tab) {
    e.stopPropagation();

    // Show confirmation dialog for terminal tabs
    const confirmed = await confirmDialogStore.confirm({
      title: 'Close Terminal',
      message:
        'Are you sure you want to close this terminal? Any running processes will be terminated.',
      confirmLabel: 'Close',
      cancelLabel: 'Cancel',
      kind: 'warning',
    });
    if (!confirmed) {
      return;
    }

    tabStore.closeTab(tab.id);
    // Trigger terminal resize after tab close
    // Use multiple dispatches to ensure terminals catch the new size after layout settles
    setTimeout(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    }, 50);
    setTimeout(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    }, 150);
  }

  function handleAddTerminal() {
    tabStore.addTerminalTab();
    // Trigger terminal resize after adding tab
    setTimeout(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    }, 100);
  }
</script>

<div class="tab-bar">
  <div class="tabs-container">
    {#each tabs as tab, index (tab.id)}
      <div
        class="tab"
        class:active={tab.id === activeTabId}
        onclick={() => handleTabClick(tab)}
        onkeydown={(e) => e.key === 'Enter' && handleTabClick(tab)}
        role="tab"
        tabindex="0"
        title={tab.title}
        style="--tab-index: {index}"
      >
        <span class="tab-icon">
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
            <polyline points="4 17 10 11 4 5"></polyline>
            <line x1="12" y1="19" x2="20" y2="19"></line>
          </svg>
        </span>
        <span class="tab-label">{tab.title}</span>
        <button
          class="close-btn"
          onclick={(e) => handleCloseClick(e, tab)}
          title="Close"
          aria-label="Close tab"
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
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
        {#if tab.id === activeTabId}
          <div class="active-indicator"></div>
        {/if}
      </div>
    {/each}
  </div>
  <button
    class="add-btn"
    onclick={handleAddTerminal}
    title="New Terminal (⌘`)"
    aria-label="New Terminal"
  >
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
      <line x1="12" y1="5" x2="12" y2="19"></line>
      <line x1="5" y1="12" x2="19" y2="12"></line>
    </svg>
  </button>
</div>

<style>
  .tab-bar {
    height: var(--tabbar-height, 44px);
    display: flex;
    align-items: stretch;
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-color);
    user-select: none;
    position: relative;
  }

  /* Subtle gradient overlay on tab bar */
  .tab-bar::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(125, 211, 252, 0.02) 50%,
      transparent 100%
    );
    pointer-events: none;
  }

  .tabs-container {
    flex: 1;
    display: flex;
    align-items: stretch;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }

  .tabs-container::-webkit-scrollbar {
    display: none;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-3) 0 var(--space-4);
    min-width: 140px;
    max-width: 200px;
    background: transparent;
    border: none;
    border-right: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.01em;
    cursor: pointer;
    position: relative;
    transition: all var(--transition-normal);
    animation: tabSlideIn 0.3s ease backwards;
    animation-delay: calc(var(--tab-index) * 30ms);
    overflow: hidden;
  }

  @keyframes tabSlideIn {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Tab hover background effect */
  .tab::before {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(
      ellipse 100% 100% at 50% 100%,
      rgba(125, 211, 252, 0.08) 0%,
      transparent 70%
    );
    opacity: 0;
    transition: opacity var(--transition-normal);
    pointer-events: none;
  }

  .tab:hover::before {
    opacity: 1;
  }

  .tab:hover {
    background: rgba(125, 211, 252, 0.03);
    color: var(--text-primary);
  }

  .tab.active {
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .tab.active::before {
    background: radial-gradient(
      ellipse 120% 80% at 50% 100%,
      rgba(125, 211, 252, 0.1) 0%,
      transparent 60%
    );
    opacity: 1;
  }

  /* Active tab indicator - subtle with entrance animation */
  .active-indicator {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--accent-color), var(--accent2-color));
    border-radius: 2px 2px 0 0;
    opacity: 0.8;
    animation: indicatorSlide 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes indicatorSlide {
    from {
      transform: scaleX(0);
      opacity: 0;
    }
    to {
      transform: scaleX(1);
      opacity: 0.8;
    }
  }

  .tab-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.6;
    transition: all var(--transition-normal);
  }

  .tab:hover .tab-icon {
    opacity: 1;
    transform: scale(1.05);
  }

  .tab.active .tab-icon {
    opacity: 1;
  }

  .tab-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    transition: all var(--transition-fast);
  }

  .tab.active .tab-label {
    color: var(--accent-color);
  }

  .modified-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-color);
    flex-shrink: 0;
    animation: modifiedPulse 2s ease-in-out infinite;
  }

  @keyframes modifiedPulse {
    0%,
    100% {
      opacity: 0.6;
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(1.15);
    }
  }

  .close-btn {
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    opacity: 0;
    transition: all var(--transition-fast);
  }

  .tab:hover .close-btn {
    opacity: 1;
  }

  .close-btn svg {
    transition: transform var(--transition-fast);
  }

  .close-btn:hover {
    background: rgba(248, 113, 113, 0.15);
    color: var(--git-deleted);
  }

  .close-btn:hover svg {
    transform: scale(1.1);
  }

  .close-btn:active {
    transform: scale(0.9);
    transition: transform 100ms ease;
  }

  .add-btn {
    flex-shrink: 0;
    width: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-left: 1px solid var(--border-subtle);
    color: var(--text-muted);
    transition: all var(--transition-normal);
    position: relative;
    overflow: hidden;
  }

  .add-btn::before {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(circle at center, rgba(125, 211, 252, 0.1) 0%, transparent 70%);
    opacity: 0;
    transition: opacity var(--transition-normal);
  }

  .add-btn:hover::before {
    opacity: 1;
  }

  .add-btn:hover {
    color: var(--accent-color);
  }

  .add-btn:hover svg {
    transform: rotate(90deg);
    transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .add-btn svg {
    transition: all 0.3s ease;
  }

  .add-btn:active {
    transform: scale(0.95);
    transition: transform 100ms ease;
  }
</style>
