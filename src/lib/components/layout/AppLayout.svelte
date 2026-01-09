<script lang="ts">
  import { appStore } from '@/lib/stores/appStore';
  import Sidebar from '@/lib/components/layout/Sidebar.svelte';
  import MainContent from '@/lib/components/layout/MainContent.svelte';
  import StatusBar from '@/lib/components/layout/StatusBar.svelte';
</script>

<div class="app-layout">
  <div class="app-body">
    <Sidebar width={$appStore.sidebarWidth} />
    <MainContent mode={$appStore.currentMode} />
  </div>
  <StatusBar mode={$appStore.currentMode} currentFile={$appStore.currentFile} />
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
