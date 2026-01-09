<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { appStore } from '@/lib/stores/appStore';
  import { tabStore } from '@/lib/stores/tabStore';
  import Sidebar from '@/lib/components/layout/Sidebar.svelte';
  import MainContent from '@/lib/components/layout/MainContent.svelte';
  import StatusBar from '@/lib/components/layout/StatusBar.svelte';

  function handleFileSelect(path: string) {
    tabStore.addEditorTab(path);
  }

  function handleKeyDown(e: KeyboardEvent) {
    // Ctrl/Cmd + ` to add new terminal
    if ((e.ctrlKey || e.metaKey) && e.key === '`') {
      e.preventDefault();
      tabStore.addTerminalTab();
    }
    // Ctrl/Cmd + W to close current tab
    if ((e.ctrlKey || e.metaKey) && e.key === 'w') {
      e.preventDefault();
      const activeId = $tabStore.activeTabId;
      if (activeId) {
        tabStore.closeTab(activeId);
      }
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeyDown);
  });
</script>

<div class="app-layout">
  <div class="app-body">
    <Sidebar width={$appStore.sidebarWidth} onFileSelect={handleFileSelect} />
    <MainContent />
  </div>
  <StatusBar />
</div>

<style>
  .app-layout {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .app-body {
    flex: 1;
    display: flex;
    overflow: hidden;
  }
</style>
