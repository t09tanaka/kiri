<script lang="ts">
  import { tabStore, activeTab } from '@/lib/stores/tabStore';
  import { currentProjectPath } from '@/lib/stores/projectStore';
  import { Terminal } from '@/lib/components/terminal';
  import { Editor } from '@/lib/components/editor';
  import TabBar from './TabBar.svelte';

  function handleEditorModified(tabId: string, modified: boolean) {
    tabStore.setModified(tabId, modified);
  }
</script>

<main class="main-content">
  <TabBar tabs={$tabStore.tabs} activeTabId={$tabStore.activeTabId} />
  <div class="content-area">
    {#if $activeTab}
      {#if $activeTab.type === 'terminal'}
        {#key $activeTab.id}
          <Terminal tabId={$activeTab.id} cwd={$currentProjectPath} />
        {/key}
      {:else if $activeTab.type === 'editor'}
        {#key $activeTab.id}
          <Editor
            filePath={$activeTab.filePath}
            onModifiedChange={(modified) => handleEditorModified($activeTab.id, modified)}
          />
        {/key}
      {/if}
    {:else}
      <div class="no-tabs">
        <p>No tabs open</p>
        <button onclick={() => tabStore.addTerminalTab()}>Open Terminal</button>
      </div>
    {/if}
  </div>
</main>

<style>
  .main-content {
    flex: 1;
    height: 100%;
    background-color: var(--bg-primary);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .content-area {
    flex: 1;
    overflow: hidden;
  }

  .no-tabs {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
    color: var(--text-secondary);
  }

  .no-tabs button {
    padding: 8px 16px;
    background-color: var(--accent-color);
    border: none;
    border-radius: 4px;
    color: white;
    cursor: pointer;
  }

  .no-tabs button:hover {
    opacity: 0.9;
  }
</style>
