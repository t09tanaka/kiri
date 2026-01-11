<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { AppLayout, StartScreen } from '@/lib/components';
  import QuickOpen from '@/lib/components/search/QuickOpen.svelte';
  import KeyboardShortcuts from '@/lib/components/ui/KeyboardShortcuts.svelte';
  import { searchStore, isQuickOpenVisible } from '@/lib/stores/searchStore';
  import { tabStore } from '@/lib/stores/tabStore';
  import { projectStore, isProjectOpen } from '@/lib/stores/projectStore';

  let showShortcuts = $state(false);

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
      return;
    }

    // Skip if typing in an input for global shortcuts
    const target = e.target as HTMLElement;
    const isTyping =
      target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;

    // ? to show keyboard shortcuts (only when not typing and no project open)
    if (e.key === '?' && !isTyping && !e.ctrlKey && !e.metaKey && !$isProjectOpen) {
      e.preventDefault();
      showShortcuts = true;
      return;
    }

    // Cmd+/ to toggle keyboard shortcuts
    if ((e.metaKey || e.ctrlKey) && e.key === '/') {
      e.preventDefault();
      showShortcuts = !showShortcuts;
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
  <KeyboardShortcuts isOpen={showShortcuts} onClose={() => (showShortcuts = false)} />
{/if}
