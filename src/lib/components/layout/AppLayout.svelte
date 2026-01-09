<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { appStore } from '@/lib/stores/appStore';
  import Sidebar from '@/lib/components/layout/Sidebar.svelte';
  import MainContent from '@/lib/components/layout/MainContent.svelte';
  import StatusBar from '@/lib/components/layout/StatusBar.svelte';

  function handleFileSelect(path: string) {
    appStore.setCurrentFile(path);
    appStore.setMode('editor');
  }

  function toggleMode() {
    const newMode = $appStore.currentMode === 'terminal' ? 'editor' : 'terminal';
    appStore.setMode(newMode);
  }

  function handleKeyDown(e: KeyboardEvent) {
    // Ctrl/Cmd + ` to toggle mode
    if ((e.ctrlKey || e.metaKey) && e.key === '`') {
      e.preventDefault();
      toggleMode();
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
    <MainContent
      mode={$appStore.currentMode}
      currentFile={$appStore.currentFile}
      onModeToggle={toggleMode}
    />
  </div>
  <StatusBar
    mode={$appStore.currentMode}
    currentFile={$appStore.currentFile}
    onModeToggle={toggleMode}
  />
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
