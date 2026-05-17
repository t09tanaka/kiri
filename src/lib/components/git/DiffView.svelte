<script lang="ts">
  import { onDestroy } from 'svelte';
  import { gitStore } from '@/lib/stores/gitStore';
  import { getDiffId } from './diffParser';
  import { createLazyLoadAction, VisibleHeaderTracker } from './diffObservers';
  import DiffSidebar from './DiffSidebar.svelte';
  import DiffFileSection from './DiffFileSection.svelte';

  const allDiffs = $derived($gitStore.allDiffs);
  const isLoading = $derived($gitStore.isDiffsLoading);
  const additions = $derived($gitStore.repoInfo?.additions ?? 0);
  const deletions = $derived($gitStore.repoInfo?.deletions ?? 0);
  const currentVisibleFile = $derived($gitStore.currentVisibleFile);

  // Which file sections have been lazily revealed at least once. Sections
  // that have been intersected stay rendered so scrolling back doesn't
  // flash a placeholder.
  let visibleSections = $state(new Set<string>());

  const lazyLoad = createLazyLoadAction((path) => {
    visibleSections = new Set([...visibleSections, path]);
  });

  const headerTracker = new VisibleHeaderTracker((topPath) => {
    if (topPath) {
      gitStore.setCurrentVisibleFile(topPath);
    } else if (allDiffs.length > 0) {
      gitStore.setCurrentVisibleFile(allDiffs[0].path);
    }
  });
  const trackHeader = headerTracker.createAction();

  function scrollToFile(path: string) {
    const element = document.getElementById(getDiffId(path));
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }

  onDestroy(() => {
    gitStore.setCurrentVisibleFile(null);
  });

  // Reset the lazy-load reveal set whenever the diff list changes so a
  // brand-new set of files starts off-screen again.
  $effect(() => {
    if (allDiffs) {
      visibleSections = new Set<string>();
    }
  });
</script>

<div class="diff-view">
  {#if isLoading}
    <div class="loading-state">
      <svg
        class="spinner"
        width="32"
        height="32"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <circle cx="12" cy="12" r="10" stroke-opacity="0.25" />
        <path d="M12 2a10 10 0 0 1 10 10" stroke-linecap="round" />
      </svg>
      <span>Loading diffs...</span>
    </div>
  {:else if allDiffs.length > 0}
    <div class="split-layout">
      <DiffSidebar
        files={allDiffs}
        totalAdditions={additions}
        totalDeletions={deletions}
        {currentVisibleFile}
        onSelect={scrollToFile}
      />
      <div class="diff-main">
        <div class="diff-scroll">
          {#each allDiffs as fileDiff (fileDiff.path)}
            <DiffFileSection
              file={fileDiff}
              isVisible={visibleSections.has(fileDiff.path)}
              {lazyLoad}
              {trackHeader}
            />
          {/each}
        </div>
      </div>
    </div>
  {:else}
    <div class="no-selection">
      <svg
        width="48"
        height="48"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
        <polyline points="22 4 12 14.01 9 11.01"></polyline>
      </svg>
      <span class="title">No changes</span>
      <span class="subtitle">Your working directory is clean</span>
    </div>
  {/if}
</div>

<style>
  .diff-view {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
    overflow: hidden;
  }

  .split-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .diff-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .diff-scroll {
    flex: 1;
    overflow-y: auto;
    contain: strict;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    height: 100%;
    color: var(--text-muted);
    font-size: 12px;
  }

  .spinner {
    animation: spin 1s linear infinite;
    color: var(--accent-color);
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .no-selection {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    height: 100%;
    padding: var(--space-6);
    text-align: center;
  }

  .no-selection svg {
    color: var(--git-added);
    opacity: 0.3;
  }

  .no-selection .title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .no-selection .subtitle {
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
