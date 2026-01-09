<script lang="ts">
  import { onMount } from 'svelte';
  import { AppLayout } from '@/lib/components';
  import QuickOpen from '@/lib/components/search/QuickOpen.svelte';
  import { searchStore, isQuickOpenVisible } from '@/lib/stores/searchStore';
  import { tabStore } from '@/lib/stores/tabStore';

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'p') {
      e.preventDefault();
      if ($isQuickOpenVisible) {
        searchStore.closeQuickOpen();
      } else {
        searchStore.openQuickOpen();
      }
    }
  }

  function handleFileSelect(path: string) {
    tabStore.openFile(path);
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });
</script>

<AppLayout />

{#if $isQuickOpenVisible}
  <QuickOpen onSelect={handleFileSelect} />
{/if}
