<script lang="ts">
  import { tabStore, activeTab } from '@/lib/stores/tabStore';

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
</script>

<footer class="status-bar">
  <div class="status-left">
    <span class="status-item mode">
      {info.mode === 'Terminal' ? '⌨' : '📝'}
      {info.mode}
    </span>
    <span class="status-item tab-count">
      {$tabStore.tabs.length} tab{$tabStore.tabs.length !== 1 ? 's' : ''}
    </span>
  </div>
  <div class="status-right">
    {#if info.file}
      <span class="status-item file-path" title={info.file}>
        {info.file.split('/').slice(-2).join('/')}
      </span>
    {/if}
    <span class="status-item">Kiri</span>
  </div>
</footer>

<style>
  .status-bar {
    height: var(--statusbar-height);
    background-color: var(--accent-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px;
    font-size: 12px;
    color: white;
  }

  .status-left,
  .status-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .status-item {
    display: flex;
    align-items: center;
  }

  .mode {
    font-weight: 500;
  }

  .tab-count {
    opacity: 0.8;
  }

  .file-path {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
