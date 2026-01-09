<script lang="ts">
  import { searchStore, type FileSearchResult } from '@/lib/stores/searchStore';

  interface Props {
    onSelect: (path: string) => void;
  }

  let { onSelect }: Props = $props();

  let query = $state('');
  let inputRef = $state<HTMLInputElement | null>(null);

  const results = $derived($searchStore.fileResults);
  const selectedIndex = $derived($searchStore.selectedIndex);
  const isSearching = $derived($searchStore.isSearching);

  $effect(() => {
    if (inputRef) {
      inputRef.focus();
    }
  });

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    query = target.value;
    searchStore.searchFiles(query);
  }

  function handleKeyDown(e: KeyboardEvent) {
    switch (e.key) {
      case 'Escape':
        searchStore.closeQuickOpen();
        break;
      case 'ArrowUp':
        e.preventDefault();
        searchStore.selectPrevious();
        break;
      case 'ArrowDown':
        e.preventDefault();
        searchStore.selectNext();
        break;
      case 'Enter': {
        e.preventDefault();
        const selected = searchStore.getSelectedFile();
        if (selected) {
          onSelect(selected.path);
          searchStore.closeQuickOpen();
        }
        break;
      }
    }
  }

  function handleResultClick(result: FileSearchResult) {
    onSelect(result.path);
    searchStore.closeQuickOpen();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      searchStore.closeQuickOpen();
    }
  }

  function getFileIcon(name: string): string {
    const ext = name.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts':
      case 'tsx':
        return '🔷';
      case 'js':
      case 'jsx':
        return '🟨';
      case 'svelte':
        return '🔶';
      case 'json':
        return '📋';
      case 'md':
        return '📝';
      case 'css':
      case 'scss':
        return '🎨';
      case 'html':
        return '🌐';
      case 'rs':
        return '🦀';
      case 'toml':
        return '⚙️';
      default:
        return '📄';
    }
  }

  function highlightMatch(name: string, query: string): string {
    if (!query) return name;

    const lowerName = name.toLowerCase();
    const lowerQuery = query.toLowerCase();
    let result = '';
    let queryIdx = 0;

    for (let i = 0; i < name.length; i++) {
      if (queryIdx < lowerQuery.length && lowerName[i] === lowerQuery[queryIdx]) {
        result += `<mark>${name[i]}</mark>`;
        queryIdx++;
      } else {
        result += name[i];
      }
    }

    return result;
  }
</script>

<div
  class="quick-open-backdrop"
  onclick={handleBackdropClick}
  onkeydown={() => {}}
  role="button"
  tabindex="-1"
>
  <div class="quick-open">
    <div class="search-input-container">
      <input
        bind:this={inputRef}
        type="text"
        class="search-input"
        placeholder="Search files..."
        value={query}
        oninput={handleInput}
        onkeydown={handleKeyDown}
      />
      {#if isSearching}
        <span class="loading-indicator">...</span>
      {/if}
    </div>

    <div class="results">
      {#if results.length === 0 && query.length > 0 && !isSearching}
        <div class="no-results">No files found</div>
      {:else}
        {#each results as result, index (result.path)}
          <button
            class="result-item"
            class:selected={index === selectedIndex}
            onclick={() => handleResultClick(result)}
          >
            <span class="result-icon">{getFileIcon(result.name)}</span>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -- Safe: only marks added to file name -->
            <span class="result-name">{@html highlightMatch(result.name, query)}</span>
            <span class="result-path">{result.path}</span>
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .quick-open-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    padding-top: 100px;
    z-index: 1000;
  }

  .quick-open {
    width: 600px;
    max-width: 90%;
    max-height: 400px;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .search-input-container {
    display: flex;
    align-items: center;
    padding: 12px;
    border-bottom: 1px solid var(--border-color);
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 16px;
    outline: none;
  }

  .search-input::placeholder {
    color: var(--text-secondary);
  }

  .loading-indicator {
    color: var(--text-secondary);
    font-size: 14px;
  }

  .results {
    flex: 1;
    overflow-y: auto;
  }

  .no-results {
    padding: 16px;
    text-align: center;
    color: var(--text-secondary);
  }

  .result-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }

  .result-item:hover,
  .result-item.selected {
    background-color: var(--bg-tertiary);
  }

  .result-item.selected {
    background-color: var(--accent-color);
  }

  .result-icon {
    flex-shrink: 0;
    font-size: 14px;
  }

  .result-name {
    flex-shrink: 0;
    font-weight: 500;
  }

  .result-name :global(mark) {
    background-color: var(--accent-color);
    color: white;
    padding: 0 1px;
    border-radius: 2px;
  }

  .result-path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
    font-size: 11px;
    margin-left: auto;
  }
</style>
