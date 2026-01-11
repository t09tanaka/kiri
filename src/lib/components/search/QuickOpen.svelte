<script lang="ts">
  import { searchStore, type FileSearchResult } from '@/lib/stores/searchStore';
  import { onMount } from 'svelte';
  import { Spinner } from '@/lib/components/ui';

  interface Props {
    onSelect: (path: string) => void;
  }

  let { onSelect }: Props = $props();

  let query = $state('');
  let inputRef = $state<HTMLInputElement | null>(null);
  let mounted = $state(false);

  const results = $derived($searchStore.fileResults);
  const selectedIndex = $derived($searchStore.selectedIndex);
  const isSearching = $derived($searchStore.isSearching);

  onMount(() => {
    mounted = true;
  });

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
  class:mounted
  onclick={handleBackdropClick}
  onkeydown={() => {}}
  role="button"
  tabindex="-1"
>
  <div class="quick-open">
    <div class="modal-glow"></div>
    <div class="modal-content">
      <div class="search-input-container">
        <span class="search-icon">
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <circle cx="11" cy="11" r="8"></circle>
            <path d="m21 21-4.35-4.35"></path>
          </svg>
        </span>
        <input
          bind:this={inputRef}
          type="text"
          class="search-input"
          placeholder="Search files by name..."
          value={query}
          oninput={handleInput}
          onkeydown={handleKeyDown}
        />
        {#if isSearching}
          <span class="loading-indicator">
            <Spinner size="sm" />
          </span>
        {:else}
          <span class="shortcut-hint">
            <kbd>esc</kbd>
          </span>
        {/if}
      </div>

      <div class="results">
        {#if results.length === 0 && query.length > 0 && !isSearching}
          <div class="no-results">
            <div class="no-results-icon">
              <svg
                width="32"
                height="32"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <circle cx="11" cy="11" r="8"></circle>
                <path d="m21 21-4.35-4.35"></path>
                <path d="M8 8l6 6"></path>
                <path d="M14 8l-6 6"></path>
              </svg>
            </div>
            <span>No files found for "<strong>{query}</strong>"</span>
          </div>
        {:else if results.length === 0 && query.length === 0}
          <div class="empty-state">
            <div class="empty-icon">
              <svg
                width="40"
                height="40"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                <polyline points="14 2 14 8 20 8"></polyline>
              </svg>
            </div>
            <span class="empty-text">Type to search files...</span>
            <span class="empty-hint">Use fuzzy matching to find files quickly</span>
          </div>
        {:else}
          {#each results as result, index (result.path)}
            <button
              class="result-item"
              class:selected={index === selectedIndex}
              onclick={() => handleResultClick(result)}
              style="--index: {index}"
            >
              <span class="result-icon">
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.5"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                  <polyline points="14 2 14 8 20 8"></polyline>
                </svg>
              </span>
              <!-- eslint-disable-next-line svelte/no-at-html-tags -- Safe: only marks added to file name -->
              <span class="result-name">{@html highlightMatch(result.name, query)}</span>
              <span class="result-path">{result.path}</span>
              {#if index === selectedIndex}
                <span class="selected-indicator"></span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>

      <div class="quick-open-footer">
        <span class="footer-item">
          <kbd>↑</kbd><kbd>↓</kbd>
          <span>navigate</span>
        </span>
        <span class="footer-item">
          <kbd>↵</kbd>
          <span>open</span>
        </span>
        <span class="footer-item">
          <kbd>esc</kbd>
          <span>close</span>
        </span>
      </div>
    </div>
  </div>
</div>

<style>
  .quick-open-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    padding-top: 100px;
    z-index: 1000;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .quick-open-backdrop.mounted {
    opacity: 1;
  }

  .quick-open {
    position: relative;
    width: 600px;
    max-width: 90%;
    max-height: 480px;
    animation: modalSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: translateY(-20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .modal-glow {
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: calc(var(--radius-xl) + 2px);
    opacity: 0.06;
    filter: blur(5px);
    z-index: -1;
    transition: opacity 0.3s ease;
  }

  .quick-open:hover .modal-glow {
    opacity: 0.1;
  }

  .modal-content {
    background: var(--bg-glass);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-xl);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--shadow-lg);
  }

  .search-input-container {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.2);
  }

  .search-icon {
    color: var(--accent-color);
    display: flex;
    align-items: center;
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 16px;
    font-family: var(--font-sans);
    font-weight: 400;
    outline: none;
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .loading-indicator {
    display: flex;
    align-items: center;
  }

  .shortcut-hint {
    font-size: 11px;
    color: var(--text-muted);
  }

  .shortcut-hint kbd {
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
  }

  .results {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2);
    min-height: 100px;
    max-height: 320px;
  }

  .no-results {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-7);
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .no-results-icon {
    color: var(--git-deleted);
    opacity: 0.4;
    animation: noResultsShake 0.4s ease;
  }

  @keyframes noResultsShake {
    0%,
    100% {
      transform: translateX(0);
    }
    20% {
      transform: translateX(-4px);
    }
    40% {
      transform: translateX(4px);
    }
    60% {
      transform: translateX(-2px);
    }
    80% {
      transform: translateX(2px);
    }
  }

  .no-results strong {
    color: var(--accent-color);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-6);
    text-align: center;
  }

  .empty-icon {
    color: var(--accent-color);
    opacity: 0.4;
    margin-bottom: var(--space-2);
  }

  .empty-text {
    font-size: 14px;
    color: var(--text-secondary);
  }

  .empty-hint {
    color: var(--text-muted);
    font-size: 12px;
    opacity: 0.8;
  }

  .result-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-sans);
    cursor: pointer;
    text-align: left;
    transition: all var(--transition-fast);
    animation: resultFadeIn 0.2s ease backwards;
    animation-delay: calc(var(--index) * 30ms);
  }

  @keyframes resultFadeIn {
    from {
      opacity: 0;
      transform: translateX(-8px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .result-item:hover {
    background: var(--bg-tertiary);
  }

  .result-item.selected {
    background: var(--accent-subtle);
  }

  .selected-indicator {
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 20px;
    background: var(--accent-color);
    border-radius: 0 2px 2px 0;
  }

  .result-icon {
    flex-shrink: 0;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    transition: all var(--transition-fast);
  }

  .result-item.selected .result-icon,
  .result-item:hover .result-icon {
    color: var(--accent-color);
    transform: scale(1.1);
  }

  .result-name {
    flex-shrink: 0;
    font-weight: 500;
    transition: all var(--transition-fast);
  }

  .result-item:hover .result-name,
  .result-item.selected .result-name {
    color: var(--accent-color);
  }

  .result-name :global(mark) {
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    color: var(--bg-primary);
    padding: 1px 4px;
    border-radius: 3px;
    font-weight: 600;
  }

  .result-path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: 11px;
    font-family: var(--font-mono);
    margin-left: auto;
    padding-left: var(--space-3);
  }

  .quick-open-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-5);
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-top: 1px solid var(--border-subtle);
  }

  .footer-item {
    font-size: 11px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .footer-item kbd {
    padding: 2px 6px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
    box-shadow: 0 1px 0 var(--bg-primary);
    transition: all var(--transition-fast);
  }

  .footer-item:hover kbd {
    color: var(--accent-color);
    border-color: var(--accent-subtle);
    transform: translateY(-1px);
    box-shadow: 0 2px 0 var(--bg-primary);
  }

  .footer-item span {
    margin-left: 2px;
  }

  /* Result item ripple effect */
  .result-item::after {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(
      circle at var(--mouse-x, 50%) var(--mouse-y, 50%),
      rgba(125, 211, 252, 0.08) 0%,
      transparent 60%
    );
    opacity: 0;
    transition: opacity var(--transition-fast);
    pointer-events: none;
    border-radius: inherit;
  }

  .result-item:hover::after {
    opacity: 1;
  }

  /* Enhanced scrollbar for results */
  .results::-webkit-scrollbar {
    width: 6px;
  }

  .results::-webkit-scrollbar-track {
    background: transparent;
  }

  .results::-webkit-scrollbar-thumb {
    background: linear-gradient(180deg, var(--border-color), var(--border-subtle));
    border-radius: 3px;
    transition: all var(--transition-normal);
  }

  .results:hover::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.3);
  }

  /* Search input focus glow effect */
  .search-input-container {
    position: relative;
    overflow: hidden;
  }

  .search-input-container::before {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, transparent, var(--accent-color), transparent);
    transform: scaleX(0);
    transition: transform 0.3s ease;
  }

  .search-input-container:focus-within::before {
    transform: scaleX(1);
  }

  /* Selected result bounce in */
  .result-item.selected {
    animation: selectedBounce 0.3s ease;
  }

  @keyframes selectedBounce {
    0% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.01);
    }
    100% {
      transform: scale(1);
    }
  }

  /* Top border shine effect */
  .modal-content::before {
    content: '';
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent-color), transparent);
    opacity: 0.6;
    z-index: 1;
  }
</style>
