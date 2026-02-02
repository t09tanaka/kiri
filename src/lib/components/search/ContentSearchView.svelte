<script lang="ts">
  import {
    contentSearchStore,
    contentSearchResults,
    isContentSearching,
    contentSearchQuery,
    type ContentSearchResult,
    type ContentMatch,
  } from '@/lib/stores/contentSearchStore';
  import { Spinner } from '@/lib/components/ui';

  interface Props {
    onOpenFile: (path: string, line?: number) => void;
    onClose: () => void;
  }

  let { onOpenFile, onClose }: Props = $props();

  const results = $derived($contentSearchResults);
  const isSearching = $derived($isContentSearching);
  const query = $derived($contentSearchQuery);
  const store = $derived($contentSearchStore);

  // Total match count
  const totalMatches = $derived(results.reduce((sum, r) => sum + r.matches.length, 0));

  function getFileName(path: string): string {
    return path.split('/').pop() ?? path;
  }

  function getRelativePath(path: string): string {
    // Show path relative to project root with leading slash
    const projectPath = store.projectPath;
    if (projectPath && path.startsWith(projectPath)) {
      const relative = path.slice(projectPath.length);
      // Ensure leading slash to indicate project root
      return relative.startsWith('/') ? relative : '/' + relative;
    }
    return path;
  }

  function handleFileClick(index: number) {
    contentSearchStore.selectFile(index);
  }

  function handleFileDoubleClick(file: ContentSearchResult, match?: ContentMatch) {
    onOpenFile(file.path, match?.line);
    onClose();
  }

  function handleMatchClick(e: MouseEvent, fileIndex: number, matchIndex: number) {
    e.stopPropagation();
    contentSearchStore.selectFile(fileIndex);
    // Small delay to ensure file selection is applied first
    setTimeout(() => {
      const currentStore = $contentSearchStore;
      if (currentStore.selectedFileIndex === fileIndex) {
        // Update match index directly via store
        contentSearchStore.selectFile(fileIndex);
        // Navigate to the specific match
        for (let i = 0; i < matchIndex; i++) {
          contentSearchStore.selectNextMatch();
        }
      }
    }, 0);
  }

  function highlightMatch(content: string, start: number, end: number): string {
    const before = escapeHtml(content.slice(0, start));
    const match = escapeHtml(content.slice(start, end));
    const after = escapeHtml(content.slice(end));
    return `${before}<mark>${match}</mark>${after}`;
  }

  function escapeHtml(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }
</script>

<div class="content-search-view">
  {#if isSearching && results.length === 0}
    <div class="loading-state">
      <Spinner size="lg" />
      <span class="loading-text">Searching...</span>
    </div>
  {:else if query.length < 2}
    <div class="empty-state">
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
        <circle cx="11" cy="11" r="8"></circle>
        <path d="m21 21-4.35-4.35"></path>
      </svg>
      <span class="title">Search in Project</span>
      <span class="subtitle">Type at least 2 characters to search</span>
    </div>
  {:else if results.length === 0}
    <div class="empty-state">
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
        <circle cx="12" cy="12" r="10"></circle>
        <path d="M16 16s-1.5-2-4-2-4 2-4 2"></path>
        <line x1="9" y1="9" x2="9.01" y2="9"></line>
        <line x1="15" y1="9" x2="15.01" y2="9"></line>
      </svg>
      <span class="title">No results</span>
      <span class="subtitle">No matches found for "{query}"</span>
    </div>
  {:else}
    <div class="split-layout">
      <!-- File list sidebar -->
      <div class="file-sidebar">
        <div class="sidebar-header">
          <span class="sidebar-title">FILES</span>
          <span class="file-count">{results.length}</span>
        </div>
        <div class="file-list">
          {#each results as file, index (file.path)}
            <button
              class="file-item"
              class:selected={store.selectedFileIndex === index}
              onclick={() => handleFileClick(index)}
              ondblclick={() => handleFileDoubleClick(file)}
              title={file.path}
            >
              <span class="file-icon">
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                  <polyline points="14 2 14 8 20 8"></polyline>
                </svg>
              </span>
              <span class="file-item-name">{getFileName(file.path)}</span>
              <span class="match-badge">{file.matches.length}</span>
            </button>
          {/each}
        </div>
        <div class="sidebar-footer">
          <span class="total-matches">{totalMatches} matches in {results.length} files</span>
        </div>
      </div>

      <!-- Match preview area -->
      <div class="preview-main">
        {#if results[store.selectedFileIndex]}
          {@const selectedFile = results[store.selectedFileIndex]}
          <div class="preview-header">
            <span class="preview-file-name">{getFileName(selectedFile.path)}</span>
            <span class="preview-file-path">{getRelativePath(selectedFile.path)}</span>
          </div>
          <div class="match-list">
            {#each selectedFile.matches as match, matchIndex (matchIndex)}
              {@const isSelectedMatch = store.selectedMatchIndex === matchIndex}
              <button
                class="match-item"
                class:selected={isSelectedMatch}
                onclick={(e) => handleMatchClick(e, store.selectedFileIndex, matchIndex)}
                ondblclick={() => handleFileDoubleClick(selectedFile, match)}
              >
                <span class="match-line-number">{match.line}</span>
                <span class="match-content">
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -- escapeHtml sanitizes content -->
                  {@html highlightMatch(match.content, match.start, match.end)}
                </span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .content-search-view {
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

  /* File sidebar */
  .file-sidebar {
    width: 240px;
    min-width: 180px;
    max-width: 320px;
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: var(--tabbar-height, 44px);
    padding: 0 var(--space-3);
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .sidebar-title {
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }

  .file-count {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent-color);
    padding: 2px 6px;
    background: rgba(125, 211, 252, 0.1);
    border-radius: var(--radius-sm);
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2) 0;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background var(--transition-fast);
  }

  .file-item:hover {
    background: var(--bg-elevated);
  }

  .file-item.selected {
    background: var(--accent-subtle);
  }

  .file-item.selected::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--accent-color);
  }

  .file-icon {
    color: var(--text-muted);
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .file-item-name {
    font-size: 12px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .match-badge {
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    padding: 1px 5px;
    background: var(--bg-tertiary);
    border-radius: var(--radius-sm);
    min-width: 18px;
    text-align: center;
  }

  .sidebar-footer {
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-tertiary);
  }

  .total-matches {
    font-size: 10px;
    color: var(--text-muted);
  }

  /* Preview area */
  .preview-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .preview-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .preview-file-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .preview-file-path {
    flex: 1;
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .match-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2) 0;
  }

  .match-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-4);
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    transition: background var(--transition-fast);
  }

  .match-item:hover {
    background: var(--bg-elevated);
  }

  .match-item.selected {
    background: var(--accent-subtle);
  }

  .match-line-number {
    flex-shrink: 0;
    width: 40px;
    text-align: right;
    color: var(--text-muted);
    user-select: none;
    padding-right: var(--space-2);
    border-right: 1px solid var(--border-subtle);
  }

  .match-content {
    flex: 1;
    color: var(--text-primary);
    white-space: pre;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .match-content :global(mark) {
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    color: var(--bg-primary);
    padding: 1px 4px;
    border-radius: 3px;
    font-weight: 500;
  }

  /* Empty and loading states */
  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    height: 100%;
    padding: var(--space-6);
    text-align: center;
  }

  .loading-text {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .empty-state svg {
    color: var(--accent-color);
    opacity: 0.3;
  }

  .empty-state .title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .empty-state .subtitle {
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
