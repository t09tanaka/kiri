<script lang="ts">
  import { tabStore, type Tab } from '@/lib/stores/tabStore';

  interface Props {
    tabs: Tab[];
    activeTabId: string | null;
  }

  let { tabs, activeTabId }: Props = $props();

  function getTabLabel(tab: Tab): string {
    if (tab.type === 'terminal') {
      return tab.title;
    }
    return tab.filePath.split('/').pop() || 'Untitled';
  }

  function getTabIcon(tab: Tab): string {
    if (tab.type === 'terminal') {
      return '⌨';
    }
    return '📄';
  }

  function handleTabClick(tab: Tab) {
    tabStore.setActiveTab(tab.id);
  }

  function handleCloseClick(e: MouseEvent, tab: Tab) {
    e.stopPropagation();
    tabStore.closeTab(tab.id);
  }

  function handleAddTerminal() {
    tabStore.addTerminalTab();
  }
</script>

<div class="tab-bar">
  <div class="tabs-container">
    {#each tabs as tab (tab.id)}
      <div
        class="tab"
        class:active={tab.id === activeTabId}
        onclick={() => handleTabClick(tab)}
        onkeydown={(e) => e.key === 'Enter' && handleTabClick(tab)}
        role="tab"
        tabindex="0"
        title={tab.type === 'editor' ? tab.filePath : tab.title}
      >
        <span class="tab-icon">{getTabIcon(tab)}</span>
        <span class="tab-label">{getTabLabel(tab)}</span>
        {#if tab.type === 'editor' && tab.modified}
          <span class="modified-dot">●</span>
        {/if}
        <button
          class="close-btn"
          onclick={(e) => handleCloseClick(e, tab)}
          title="Close"
          aria-label="Close tab"
        >
          ×
        </button>
      </div>
    {/each}
  </div>
  <button
    class="add-btn"
    onclick={handleAddTerminal}
    title="New Terminal"
    aria-label="New Terminal"
  >
    +
  </button>
</div>

<style>
  .tab-bar {
    height: 35px;
    display: flex;
    align-items: stretch;
    background-color: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-color);
  }

  .tabs-container {
    flex: 1;
    display: flex;
    align-items: stretch;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .tabs-container::-webkit-scrollbar {
    height: 4px;
  }

  .tabs-container::-webkit-scrollbar-thumb {
    background-color: var(--border-color);
    border-radius: 2px;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 12px;
    min-width: 100px;
    max-width: 180px;
    background: none;
    border: none;
    border-right: 1px solid var(--border-color);
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
    transition:
      background-color 0.15s,
      color 0.15s;
  }

  .tab:hover {
    background-color: var(--bg-secondary);
    color: var(--text-primary);
  }

  .tab.active {
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border-bottom: 2px solid var(--accent-color);
    margin-bottom: -1px;
  }

  .tab-icon {
    flex-shrink: 0;
    font-size: 14px;
  }

  .tab-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: left;
  }

  .modified-dot {
    color: var(--accent-color);
    font-size: 10px;
  }

  .close-btn {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    padding: 0;
    background: none;
    border: none;
    border-radius: 3px;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
    opacity: 0;
    transition:
      opacity 0.15s,
      background-color 0.15s;
  }

  .tab:hover .close-btn {
    opacity: 1;
  }

  .close-btn:hover {
    background-color: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
  }

  .add-btn {
    flex-shrink: 0;
    width: 35px;
    background: none;
    border: none;
    border-left: 1px solid var(--border-color);
    color: var(--text-secondary);
    font-size: 18px;
    cursor: pointer;
    transition:
      background-color 0.15s,
      color 0.15s;
  }

  .add-btn:hover {
    background-color: var(--bg-secondary);
    color: var(--text-primary);
  }
</style>
