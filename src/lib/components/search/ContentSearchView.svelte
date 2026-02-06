<script lang="ts">
  import {
    contentSearchStore,
    contentSearchResults,
    isContentSearching,
    contentSearchQuery,
    type ContentSearchResult,
    type ContentMatch,
  } from '@/lib/stores/contentSearchStore';
  import { fileService } from '@/lib/services/fileService';
  import { Spinner } from '@/lib/components/ui';
  import {
    escapeHtml,
    getLanguageFromPath,
    getLineLanguage,
    highlightLine,
    insertMarksIntoHighlightedHtml,
    supportsEmbeddedLanguages,
    type EmbeddedContext,
  } from '@/lib/utils/syntaxHighlight';

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

  // File content cache
  let fileContentCache = $state<Map<string, string[]>>(new Map());
  let loadingFile = $state<string | null>(null);

  // Expanded files in sidebar
  let expandedFiles = $state<Set<string>>(new Set());

  // Reference to main content container for scrolling
  let mainContentRef: HTMLDivElement | null = $state(null);

  // Highlighted line cache (non-reactive, used for rendering only)
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- intentionally non-reactive cache
  const highlightedLineCache = new Map<string, string[]>();

  function getFileName(path: string): string {
    return path.split('/').pop() ?? path;
  }

  function getRelativePath(path: string): string {
    const projectPath = store.projectPath;
    if (projectPath && path.startsWith(projectPath)) {
      const relative = path.slice(projectPath.length);
      return relative.startsWith('/') ? relative : '/' + relative;
    }
    return path;
  }

  async function loadFileContent(path: string) {
    if (fileContentCache.has(path)) return;

    loadingFile = path;
    try {
      const content = await fileService.readFile(path);
      const lines = content.split('\n');
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      fileContentCache = new Map(fileContentCache).set(path, lines);

      // Pre-compute syntax highlighting for all lines
      const baseLanguage = getLanguageFromPath(path);
      const hasEmbedded = supportsEmbeddedLanguages(path);
      const highlighted: string[] = [];
      let context: EmbeddedContext = 'template';

      for (const line of lines) {
        let language = baseLanguage;
        if (hasEmbedded) {
          const result = getLineLanguage(line, baseLanguage, context);
          language = result.language;
          context = result.newContext;
        }
        highlighted.push(highlightLine(line, language));
      }

      highlightedLineCache.set(path, highlighted);
    } catch (error) {
      console.error('Failed to load file:', error);
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      fileContentCache = new Map(fileContentCache).set(path, ['// Failed to load file']);
    } finally {
      loadingFile = null;
    }
  }

  function handleFileClick(index: number) {
    const file = results[index];
    if (file) {
      contentSearchStore.selectFile(index);
      loadFileContent(file.path);
      // Toggle expand
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      const newExpanded = new Set(expandedFiles);
      if (newExpanded.has(file.path)) {
        newExpanded.delete(file.path);
      } else {
        newExpanded.add(file.path);
      }
      expandedFiles = newExpanded;
    }
  }

  function handleMatchClick(fileIndex: number, matchIndex: number, line: number) {
    contentSearchStore.selectFile(fileIndex);
    // Update match index
    const currentState = $contentSearchStore;
    if (currentState.selectedFileIndex === fileIndex) {
      // Set match index directly through multiple calls
      for (let i = 0; i < matchIndex; i++) {
        contentSearchStore.selectNextMatch();
      }
    }

    // Scroll to line in main content
    scrollToLine(line);
  }

  function scrollToLine(line: number) {
    if (!mainContentRef) return;

    const lineElement = mainContentRef.querySelector(`[data-line="${line}"]`);
    if (lineElement) {
      lineElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }

  function handleFileDoubleClick(file: ContentSearchResult, match?: ContentMatch) {
    onOpenFile(file.path, match?.line);
    onClose();
  }

  function renderLine(
    filePath: string,
    line: string,
    lineNumber: number,
    matches: ContentMatch[]
  ): string {
    const highlightedLines = highlightedLineCache.get(filePath);
    const highlightedHtml = highlightedLines?.[lineNumber - 1] ?? escapeHtml(line);

    const lineMatches = matches.filter((m) => m.line === lineNumber);
    if (lineMatches.length === 0) {
      return highlightedHtml;
    }

    const marks = lineMatches.map((m) => ({ start: m.start, end: m.end }));
    return insertMarksIntoHighlightedHtml(highlightedHtml, marks);
  }

  // Load file content when selected file changes
  $effect(() => {
    const selectedFile = results[store.selectedFileIndex];
    if (selectedFile && !fileContentCache.has(selectedFile.path)) {
      loadFileContent(selectedFile.path);
    }
  });

  // Auto-expand all files when results change
  $effect(() => {
    if (results.length > 0) {
      expandedFiles = new Set(results.map((r) => r.path));
    }
  });
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
      <!-- File tree sidebar -->
      <div class="file-sidebar">
        <div class="sidebar-header">
          <span class="sidebar-title">RESULTS</span>
          <span class="file-count">{totalMatches}</span>
        </div>
        <div class="file-tree">
          {#each results as file, fileIndex (file.path)}
            {@const isExpanded = expandedFiles.has(file.path)}
            {@const isSelected = store.selectedFileIndex === fileIndex}
            <div class="tree-file">
              <button
                class="tree-file-header"
                class:selected={isSelected}
                onclick={() => handleFileClick(fileIndex)}
                ondblclick={() => handleFileDoubleClick(file)}
                title={getRelativePath(file.path)}
              >
                <span class="expand-icon" class:expanded={isExpanded}>
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M8 5l8 7-8 7z"></path>
                  </svg>
                </span>
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
                <span class="tree-file-name">{getFileName(file.path)}</span>
                <span class="match-badge">{file.matches.length}</span>
              </button>

              {#if isExpanded}
                <div class="tree-matches">
                  {#each file.matches as match, matchIndex (matchIndex)}
                    {@const isMatchSelected = isSelected && store.selectedMatchIndex === matchIndex}
                    <button
                      class="tree-match-item"
                      class:selected={isMatchSelected}
                      onclick={() => handleMatchClick(fileIndex, matchIndex, match.line)}
                      ondblclick={() => handleFileDoubleClick(file, match)}
                    >
                      <span class="match-line-num">{match.line}</span>
                      <span class="match-preview">{match.content.trim()}</span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        </div>
        <div class="sidebar-footer">
          <span class="total-matches">{totalMatches} matches in {results.length} files</span>
        </div>
      </div>

      <!-- File content preview -->
      <div class="preview-main">
        {#if results[store.selectedFileIndex]}
          {@const selectedFile = results[store.selectedFileIndex]}
          {@const fileLines = fileContentCache.get(selectedFile.path) ?? []}
          <div class="preview-header">
            <span class="preview-file-name">{getFileName(selectedFile.path)}</span>
            <span class="preview-file-path">{getRelativePath(selectedFile.path)}</span>
            <button
              class="open-file-btn"
              onclick={() => handleFileDoubleClick(selectedFile)}
              title="Open in editor"
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
                <polyline points="15 3 21 3 21 9"></polyline>
                <line x1="10" y1="14" x2="21" y2="3"></line>
              </svg>
            </button>
          </div>
          <div class="file-content" bind:this={mainContentRef}>
            {#if loadingFile === selectedFile.path}
              <div class="loading-content">
                <Spinner size="sm" />
                <span>Loading file...</span>
              </div>
            {:else if fileLines.length > 0}
              {#each fileLines as line, lineIndex (lineIndex)}
                {@const lineNumber = lineIndex + 1}
                {@const hasMatch = selectedFile.matches.some((m) => m.line === lineNumber)}
                <div class="code-line" class:has-match={hasMatch} data-line={lineNumber}>
                  <span class="line-number">{lineNumber}</span>
                  <span class="line-content">
                    <!-- eslint-disable-next-line svelte/no-at-html-tags -- escapeHtml sanitizes content -->
                    {@html renderLine(selectedFile.path, line, lineNumber, selectedFile.matches)}
                  </span>
                </div>
              {/each}
            {:else}
              <div class="empty-file">
                <span>Empty file or unable to load content</span>
              </div>
            {/if}
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
    width: 300px;
    min-width: 200px;
    max-width: 400px;
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

  .file-tree {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2) 0;
  }

  .tree-file {
    margin-bottom: 2px;
  }

  .tree-file-header {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    padding: var(--space-1) var(--space-2);
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background var(--transition-fast);
    font-size: 12px;
  }

  .tree-file-header:hover {
    background: var(--bg-elevated);
  }

  .tree-file-header.selected {
    background: var(--accent-subtle);
  }

  .expand-icon {
    color: var(--text-muted);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    transition: transform var(--transition-fast);
  }

  .expand-icon.expanded {
    transform: rotate(90deg);
  }

  .file-icon {
    color: var(--text-muted);
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .tree-file-name {
    flex: 1;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .tree-matches {
    padding-left: var(--space-5);
  }

  .tree-match-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-1) var(--space-2);
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    font-size: 11px;
    font-family: var(--font-mono);
    transition: background var(--transition-fast);
  }

  .tree-match-item:hover {
    background: var(--bg-elevated);
  }

  .tree-match-item.selected {
    background: var(--accent-subtle);
  }

  .match-line-num {
    color: var(--text-muted);
    min-width: 32px;
    text-align: right;
    flex-shrink: 0;
  }

  .match-preview {
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
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
    min-width: 0;
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

  .open-file-btn {
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

  .open-file-btn:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
  }

  .file-content {
    flex: 1;
    overflow: auto;
    background: var(--bg-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
  }

  .loading-content {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-6);
    color: var(--text-muted);
  }

  .empty-file {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    color: var(--text-muted);
  }

  .code-line {
    display: flex;
    min-height: 1.6em;
    padding: 0 var(--space-2);
  }

  .code-line.has-match {
    background: rgba(125, 211, 252, 0.08);
  }

  .line-number {
    flex-shrink: 0;
    width: 48px;
    padding-right: var(--space-3);
    text-align: right;
    color: var(--text-muted);
    user-select: none;
    border-right: 1px solid var(--border-subtle);
    margin-right: var(--space-3);
  }

  .line-content {
    flex: 1;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .line-content :global(mark) {
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    color: var(--bg-primary);
    padding: 1px 4px;
    border-radius: 3px;
    font-weight: 500;
  }

  .line-content :global(mark span) {
    color: inherit;
  }

  /* Syntax highlighting colors */
  .line-content :global(.hljs-keyword),
  .line-content :global(.hljs-selector-tag),
  .line-content :global(.hljs-built_in),
  .line-content :global(.hljs-name) {
    color: #c792ea;
  }

  .line-content :global(.hljs-string),
  .line-content :global(.hljs-selector-attr),
  .line-content :global(.hljs-selector-pseudo),
  .line-content :global(.hljs-addition) {
    color: #c3e88d;
  }

  .line-content :global(.hljs-number),
  .line-content :global(.hljs-literal) {
    color: #f78c6c;
  }

  .line-content :global(.hljs-function),
  .line-content :global(.hljs-title) {
    color: #82aaff;
  }

  .line-content :global(.hljs-comment),
  .line-content :global(.hljs-quote) {
    color: #546e7a;
    font-style: italic;
  }

  .line-content :global(.hljs-tag) {
    color: #f07178;
  }

  .line-content :global(.hljs-attr),
  .line-content :global(.hljs-attribute) {
    color: #ffcb6b;
  }

  .line-content :global(.hljs-variable),
  .line-content :global(.hljs-template-variable) {
    color: #f07178;
  }

  .line-content :global(.hljs-type),
  .line-content :global(.hljs-class .hljs-title) {
    color: #ffcb6b;
  }

  .line-content :global(.hljs-params) {
    color: #89ddff;
  }

  .line-content :global(.hljs-regexp) {
    color: #89ddff;
  }

  .line-content :global(.hljs-symbol),
  .line-content :global(.hljs-bullet) {
    color: #89ddff;
  }

  .line-content :global(.hljs-meta) {
    color: #ffcb6b;
  }

  .line-content :global(.hljs-deletion) {
    color: #f07178;
  }

  .line-content :global(.hljs-punctuation) {
    color: #89ddff;
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
