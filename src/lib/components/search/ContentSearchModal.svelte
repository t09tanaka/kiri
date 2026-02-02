<script lang="ts">
  import ContentSearchView from './ContentSearchView.svelte';
  import SearchSettingsPanel from './SearchSettingsPanel.svelte';
  import { contentSearchStore, isContentSearchSettingsOpen } from '@/lib/stores/contentSearchStore';
  import { onMount, onDestroy } from 'svelte';
  import { Spinner } from '@/lib/components/ui';

  interface Props {
    onOpenFile: (path: string, line?: number) => void;
    onClose: () => void;
  }

  let { onOpenFile, onClose }: Props = $props();

  let mounted = $state(false);
  let searchInput: HTMLTextAreaElement | null = $state(null);

  const store = $derived($contentSearchStore);
  const isSettingsOpen = $derived($isContentSearchSettingsOpen);

  onMount(() => {
    mounted = true;
    // Use capture phase to intercept before terminal handles it
    document.addEventListener('keydown', handleKeyDown, true);

    // Focus search input
    setTimeout(() => {
      searchInput?.focus();
    }, 100);
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();

      // Close settings first if open
      if (isSettingsOpen) {
        contentSearchStore.closeSettings();
      } else {
        onClose();
      }
      return;
    }

    // Navigate between files
    if (e.key === 'ArrowDown' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      contentSearchStore.selectNextFile();
      return;
    }

    if (e.key === 'ArrowUp' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      contentSearchStore.selectPreviousFile();
      return;
    }

    // Navigate between matches within a file
    if (e.key === 'ArrowDown' && e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      contentSearchStore.selectNextMatch();
      return;
    }

    if (e.key === 'ArrowUp' && e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      contentSearchStore.selectPreviousMatch();
      return;
    }

    // Open selected file (Enter without Shift)
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();

      const file = contentSearchStore.getSelectedFile();
      const match = contentSearchStore.getSelectedMatch();
      if (file) {
        onOpenFile(file.path, match?.line);
        onClose();
      }
      return;
    }
    // Shift+Enter allows newline in textarea (don't prevent default)
  }

  function handleSearchInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    contentSearchStore.search(target.value);
    // Auto-resize textarea
    target.style.height = 'auto';
    target.style.height = Math.min(target.scrollHeight, 120) + 'px';
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function toggleSettings() {
    contentSearchStore.toggleSettings();
  }
</script>

<div
  class="content-search-backdrop"
  class:mounted
  onclick={handleBackdropClick}
  onkeydown={() => {}}
  role="button"
  tabindex="-1"
>
  <div class="content-search-modal">
    <div class="modal-glow"></div>
    <div class="modal-content">
      <div class="modal-header">
        <div class="search-container">
          <svg
            class="search-icon"
            width="16"
            height="16"
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
          <textarea
            bind:this={searchInput}
            class="search-input"
            placeholder="Search in project..."
            rows="1"
            oninput={handleSearchInput}
            spellcheck="false"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off">{store.query}</textarea
          >
          {#if store.isSearching}
            <div class="search-spinner">
              <Spinner size="sm" />
            </div>
          {/if}
        </div>
        <div class="header-actions">
          <button
            class="action-btn settings-btn"
            class:active={isSettingsOpen}
            onclick={toggleSettings}
            title="Search settings"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="3"></circle>
              <path
                d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
              ></path>
            </svg>
          </button>
          <button class="action-btn close-btn" onclick={onClose} title="Close (Esc)">
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
      </div>

      <div class="modal-body">
        {#if isSettingsOpen}
          <SearchSettingsPanel />
        {:else}
          <ContentSearchView {onOpenFile} onClose={() => onClose()} />
        {/if}
      </div>

      <div class="modal-footer">
        <span class="footer-item">
          <kbd>↵</kbd>
          <span>open</span>
        </span>
        <span class="footer-item">
          <kbd>⇧↵</kbd>
          <span>newline</span>
        </span>
        <span class="footer-item">
          <kbd>↑</kbd><kbd>↓</kbd>
          <span>navigate files</span>
        </span>
        <span class="footer-item">
          <kbd>⇧↑</kbd><kbd>⇧↓</kbd>
          <span>navigate matches</span>
        </span>
        <span class="footer-item">
          <kbd>Esc</kbd>
          <span>close</span>
        </span>
      </div>
    </div>
  </div>
</div>

<style>
  .content-search-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .content-search-backdrop.mounted {
    opacity: 1;
  }

  .content-search-modal {
    position: relative;
    width: 90%;
    max-width: 1200px;
    height: 85%;
    max-height: 900px;
    min-height: 400px;
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

  .content-search-modal:hover .modal-glow {
    opacity: 0.1;
  }

  .modal-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-glass);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-xl);
    overflow: hidden;
    box-shadow: var(--shadow-lg);
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

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid var(--border-color);
    gap: var(--space-3);
  }

  .search-container {
    flex: 1;
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    min-height: 36px;
    max-height: 140px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
    transition:
      border-color var(--transition-fast),
      box-shadow var(--transition-fast),
      background var(--transition-fast);
  }

  .search-container:focus-within {
    border-color: var(--accent-color);
    box-shadow: 0 0 0 3px rgba(125, 211, 252, 0.15);
    background: rgba(125, 211, 252, 0.05);
  }

  .search-container:focus-within .search-icon {
    color: var(--accent-color);
  }

  .search-icon {
    color: var(--text-muted);
    flex-shrink: 0;
    margin-top: 2px;
    transition: color var(--transition-fast);
  }

  .search-input {
    flex: 1;
    min-height: 20px;
    max-height: 120px;
    background: transparent;
    border: none;
    outline: none;
    box-shadow: none;
    color: var(--text-primary);
    font-size: 14px;
    font-family: var(--font-sans);
    line-height: 1.4;
    resize: none;
    overflow-y: auto;
    -webkit-appearance: none;
    appearance: none;
  }

  .search-input:focus {
    border: none;
    outline: none;
    box-shadow: none;
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .search-spinner {
    flex-shrink: 0;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .action-btn:hover:not(:disabled) {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
  }

  .action-btn.settings-btn.active {
    background: rgba(125, 211, 252, 0.15);
    color: var(--accent-color);
  }

  .action-btn.close-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .modal-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .modal-footer {
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
  }

  .footer-item span {
    margin-left: 2px;
  }
</style>
