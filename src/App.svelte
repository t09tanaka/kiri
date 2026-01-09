<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { AppLayout, StartScreen } from '@/lib/components';
  import QuickOpen from '@/lib/components/search/QuickOpen.svelte';
  import { searchStore, isQuickOpenVisible } from '@/lib/stores/searchStore';
  import { tabStore } from '@/lib/stores/tabStore';
  import { projectStore, isProjectOpen } from '@/lib/stores/projectStore';

  async function handleOpenDirectory() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Open Directory',
    });

    if (selected && typeof selected === 'string') {
      await projectStore.openProject(selected);
    }
  }

  async function handleKeyDown(e: KeyboardEvent) {
    // Cmd+O: Open directory
    if ((e.metaKey || e.ctrlKey) && e.key === 'o') {
      e.preventDefault();
      await handleOpenDirectory();
      return;
    }

    // Cmd+Shift+W: Close project (return to start screen)
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'w') {
      e.preventDefault();
      projectStore.closeProject();
      return;
    }

    // Cmd+P: Quick open (only when project is open)
    if ((e.metaKey || e.ctrlKey) && e.key === 'p' && $isProjectOpen) {
      e.preventDefault();
      if ($isQuickOpenVisible) {
        searchStore.closeQuickOpen();
      } else {
        const path = projectStore.getCurrentPath();
        if (path) {
          searchStore.setRootPath(path);
        }
        searchStore.openQuickOpen();
      }
      return;
    }

    // Cmd+Shift+N: New window
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'n') {
      e.preventDefault();
      try {
        await invoke('create_window');
      } catch (error) {
        console.error('Failed to create window:', error);
      }
    }
  }

  function handleFileSelect(path: string) {
    tabStore.openFile(path);
  }

  onMount(() => {
    projectStore.init();
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });
</script>

{#if $isProjectOpen}
  <AppLayout />

  {#if $isQuickOpenVisible}
    <QuickOpen onSelect={handleFileSelect} />
  {/if}
{:else}
  <StartScreen />
{/if}
