<script lang="ts">
  import { gitStore, getStatusIcon, getStatusColor } from '@/lib/stores/gitStore';
  import { onDestroy } from 'svelte';

  const allDiffs = $derived($gitStore.allDiffs);
  const isLoading = $derived($gitStore.isDiffsLoading);
  const additions = $derived($gitStore.repoInfo?.additions ?? 0);
  const deletions = $derived($gitStore.repoInfo?.deletions ?? 0);
  const currentVisibleFile = $derived($gitStore.currentVisibleFile);

  // Track which sections are visible and should render their content
  // Use $state to trigger reactivity when the set changes
  let visibleSections = $state(new Set<string>());

  // Track visible file headers for active state (non-reactive, used only in $effect)
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- intentionally non-reactive
  const visibleHeaders = new Map<string, number>();

  interface DiffLine {
    type: 'add' | 'remove' | 'context' | 'header';
    content: string;
    lineNumber: number | null;
  }

  // Cache parsed diffs to avoid re-parsing (non-reactive cache)
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- intentionally non-reactive to avoid state_unsafe_mutation
  const parsedDiffCache = new Map<string, DiffLine[]>();

  function parseDiff(path: string, diffContent: string): DiffLine[] {
    if (parsedDiffCache.has(path)) {
      return parsedDiffCache.get(path)!;
    }

    if (!diffContent) {
      const result: DiffLine[] = [];
      parsedDiffCache.set(path, result);
      return result;
    }

    const lines: DiffLine[] = [];
    let lineNum = 0;

    for (const line of diffContent.split('\n')) {
      if (line.startsWith('+ ')) {
        lineNum++;
        lines.push({
          type: 'add',
          content: line.slice(2),
          lineNumber: lineNum,
        });
      } else if (line.startsWith('- ')) {
        lines.push({
          type: 'remove',
          content: line.slice(2),
          lineNumber: null,
        });
      } else if (line.startsWith('  ')) {
        lineNum++;
        lines.push({
          type: 'context',
          content: line.slice(2),
          lineNumber: lineNum,
        });
      } else if (line.startsWith('@@')) {
        const match = line.match(/@@ -\d+(?:,\d+)? \+(\d+)/);
        if (match) {
          lineNum = parseInt(match[1], 10) - 1;
        }
        lines.push({
          type: 'header',
          content: line,
          lineNumber: null,
        });
      }
    }

    parsedDiffCache.set(path, lines);
    return lines;
  }

  function getFileName(path: string): string {
    return path.split('/').pop() ?? path;
  }

  function getImageMimeType(path: string): string {
    const ext = path.split('.').pop()?.toLowerCase() ?? '';
    const mimeTypes: Record<string, string> = {
      png: 'png',
      jpg: 'jpeg',
      jpeg: 'jpeg',
      gif: 'gif',
      ico: 'x-icon',
      webp: 'webp',
      bmp: 'bmp',
      svg: 'svg+xml',
      tiff: 'tiff',
      tif: 'tiff',
    };
    return mimeTypes[ext] ?? 'png';
  }

  function getDiffId(path: string): string {
    return `diff-${path.replace(/[^a-zA-Z0-9]/g, '-')}`;
  }

  function scrollToFile(path: string) {
    const element = document.getElementById(getDiffId(path));
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }

  // Count lines for placeholder height estimation
  function estimateLineCount(diff: string): number {
    if (!diff) return 3;
    return Math.max(3, diff.split('\n').length);
  }

  // Intersection Observer action for lazy loading
  function lazyLoad(node: HTMLElement, path: string) {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            // Create a new Set to trigger reactivity
            visibleSections = new Set([...visibleSections, path]);
            // Once visible, no need to observe anymore
            observer.unobserve(node);
          }
        });
      },
      {
        root: null,
        rootMargin: '200px', // Start loading 200px before visible
        threshold: 0,
      }
    );

    observer.observe(node);

    return {
      destroy() {
        observer.disconnect();
      },
    };
  }

  // Helper function to update the topmost visible file in the store
  function updateCurrentVisibleFile() {
    if (visibleHeaders.size > 0) {
      // Find the header closest to the top (smallest positive value or largest negative)
      let topFile: string | null = null;
      let minTop = Infinity;

      for (const [path, top] of visibleHeaders) {
        if (top < minTop) {
          minTop = top;
          topFile = path;
        }
      }

      gitStore.setCurrentVisibleFile(topFile);
    } else if (allDiffs.length > 0) {
      // Default to first file if none visible
      gitStore.setCurrentVisibleFile(allDiffs[0].path);
    }
  }

  // Track which file header is at the top of the viewport
  function trackHeader(node: HTMLElement, path: string) {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            // Store the top position of this header
            visibleHeaders.set(path, entry.boundingClientRect.top);
          } else {
            // Remove from visible headers
            visibleHeaders.delete(path);
          }
        });
        // Update the current visible file after each intersection change
        updateCurrentVisibleFile();
      },
      {
        root: null,
        rootMargin: '-44px 0px -80% 0px', // Top 20% of viewport after header
        threshold: 0,
      }
    );

    observer.observe(node);

    return {
      destroy() {
        observer.disconnect();
      },
    };
  }

  // Clear on unmount
  onDestroy(() => {
    gitStore.setCurrentVisibleFile(null);
  });

  // Clear cache when diffs change
  $effect(() => {
    if (allDiffs) {
      parsedDiffCache.clear();
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
      <!-- File list sidebar -->
      <div class="file-sidebar">
        <div class="sidebar-header">
          <span class="sidebar-title">FILES</span>
          <span class="file-stats">
            <span class="stat-additions">+{additions}</span>
            <span class="stat-deletions">-{deletions}</span>
          </span>
        </div>
        <div class="file-list">
          {#each allDiffs as fileDiff (fileDiff.path)}
            <button
              class="file-item"
              class:selected={currentVisibleFile === fileDiff.path}
              onclick={() => scrollToFile(fileDiff.path)}
              title={fileDiff.path}
            >
              <span class="file-status" style="color: {getStatusColor(fileDiff.status)}">
                {getStatusIcon(fileDiff.status)}
              </span>
              <span class="file-item-name">{getFileName(fileDiff.path)}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Diff content area -->
      <div class="diff-main">
        <div class="diff-scroll">
          {#each allDiffs as fileDiff (fileDiff.path)}
            <div class="file-section" id={getDiffId(fileDiff.path)} use:lazyLoad={fileDiff.path}>
              <div class="file-header" use:trackHeader={fileDiff.path}>
                <span class="status-badge" style="color: {getStatusColor(fileDiff.status)}">
                  {getStatusIcon(fileDiff.status)}
                </span>
                <span class="file-name">{getFileName(fileDiff.path)}</span>
                <span class="file-path">{fileDiff.path}</span>
              </div>

              <div class="diff-content">
                {#if visibleSections.has(fileDiff.path)}
                  {#if fileDiff.is_binary}
                    <!-- Binary/Image file display -->
                    <div class="binary-diff">
                      {#if fileDiff.original_content_base64 || fileDiff.current_content_base64}
                        <div class="image-comparison">
                          {#if fileDiff.original_content_base64}
                            <div class="image-panel original">
                              <div class="image-label">Original</div>
                              <img
                                src="data:image/{getImageMimeType(
                                  fileDiff.path
                                )};base64,{fileDiff.original_content_base64}"
                                alt="Original: {fileDiff.path}"
                              />
                            </div>
                          {/if}
                          {#if fileDiff.current_content_base64}
                            <div class="image-panel current">
                              <div class="image-label">
                                {fileDiff.original_content_base64 ? 'Current' : 'New'}
                              </div>
                              <img
                                src="data:image/{getImageMimeType(
                                  fileDiff.path
                                )};base64,{fileDiff.current_content_base64}"
                                alt="Current: {fileDiff.path}"
                              />
                            </div>
                          {/if}
                        </div>
                      {:else}
                        <div class="binary-notice">
                          <svg
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                          >
                            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                            <circle cx="8.5" cy="8.5" r="1.5"></circle>
                            <polyline points="21 15 16 10 5 21"></polyline>
                          </svg>
                          <span>Binary file changed</span>
                        </div>
                      {/if}
                    </div>
                  {:else}
                    {@const lines = parseDiff(fileDiff.path, fileDiff.diff)}
                    {#if lines.length > 0}
                      {#each lines as line, index (index)}
                        <div class="diff-line {line.type}">
                          <span class="line-number">
                            {line.lineNumber ?? ''}
                          </span>
                          <span class="line-prefix">
                            {#if line.type === 'add'}+{:else if line.type === 'remove'}-{:else if line.type === 'context'}&nbsp;{/if}
                          </span>
                          <span class="line-content">{line.content}</span>
                        </div>
                      {/each}
                    {:else}
                      <div class="no-diff">
                        <span>No visible changes</span>
                      </div>
                    {/if}
                  {/if}
                {:else}
                  <!-- Placeholder while not visible -->
                  <div
                    class="diff-placeholder"
                    style="height: {fileDiff.is_binary
                      ? 200
                      : estimateLineCount(fileDiff.diff) * 22}px"
                  >
                    <span class="placeholder-text">Scroll to load diff...</span>
                  </div>
                {/if}
              </div>
            </div>
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

  /* File sidebar */
  .file-sidebar {
    width: 200px;
    min-width: 150px;
    max-width: 300px;
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

  .file-stats {
    display: flex;
    gap: var(--space-2);
    font-size: 10px;
    font-weight: 600;
  }

  .stat-additions {
    color: var(--git-added);
  }

  .stat-deletions {
    color: var(--git-deleted);
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

  .file-status {
    font-size: 10px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .file-item-name {
    font-size: 12px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Diff main area */
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

  .file-section {
    border-bottom: 1px solid var(--border-color);
    contain: layout style;
  }

  .file-section:last-child {
    border-bottom: none;
  }

  .file-header {
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-subtle);
  }

  .status-badge {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid currentColor;
    background: transparent;
  }

  .file-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .file-path {
    flex: 1;
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff-content {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
    contain: content;
  }

  .diff-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-tertiary);
    opacity: 0.5;
  }

  .placeholder-text {
    font-size: 11px;
    color: var(--text-muted);
  }

  .diff-line {
    display: flex;
    min-height: 22px;
  }

  .diff-line.add {
    background: rgba(74, 222, 128, 0.1);
  }

  .diff-line.add .line-prefix {
    color: var(--git-added);
  }

  .diff-line.remove {
    background: rgba(248, 113, 113, 0.1);
  }

  .diff-line.remove .line-prefix {
    color: var(--git-deleted);
  }

  .diff-line.context {
    background: transparent;
  }

  .diff-line.header {
    background: var(--bg-tertiary);
    color: var(--text-muted);
    font-size: 11px;
    padding: var(--space-2) 0;
    margin: var(--space-2) 0;
    border-top: 1px solid var(--border-subtle);
    border-bottom: 1px solid var(--border-subtle);
  }

  .diff-line.header .line-content {
    color: var(--accent-color);
    opacity: 0.7;
  }

  .line-number {
    flex-shrink: 0;
    width: 48px;
    padding: 0 var(--space-2);
    text-align: right;
    color: var(--text-muted);
    user-select: none;
    border-right: 1px solid var(--border-subtle);
    opacity: 0.6;
  }

  .line-prefix {
    flex-shrink: 0;
    width: 20px;
    padding: 0 var(--space-1);
    text-align: center;
    user-select: none;
    font-weight: 600;
  }

  .line-content {
    flex: 1;
    padding: 0 var(--space-2);
    white-space: pre;
    color: var(--text-primary);
  }

  .no-diff {
    padding: var(--space-4);
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
  }

  /* Binary/Image diff styles */
  .binary-diff {
    padding: var(--space-4);
  }

  .image-comparison {
    display: flex;
    gap: var(--space-4);
    justify-content: center;
    flex-wrap: wrap;
  }

  .image-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--bg-tertiary);
    max-width: 400px;
  }

  .image-panel.original {
    border: 1px solid var(--git-deleted);
  }

  .image-panel.current {
    border: 1px solid var(--git-added);
  }

  .image-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .image-panel.original .image-label {
    color: var(--git-deleted);
  }

  .image-panel.current .image-label {
    color: var(--git-added);
  }

  .image-panel img {
    max-width: 100%;
    max-height: 300px;
    object-fit: contain;
    border-radius: var(--radius-sm);
    background: repeating-conic-gradient(#808080 0% 25%, transparent 0% 50%) 50% / 16px 16px;
  }

  .binary-notice {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-6);
    color: var(--text-muted);
    font-size: 12px;
  }

  .binary-notice svg {
    opacity: 0.5;
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
