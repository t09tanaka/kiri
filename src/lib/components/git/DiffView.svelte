<script lang="ts">
  import { gitStore, getStatusIcon, getStatusColor } from '@/lib/stores/gitStore';
  import { onDestroy } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';

  const allDiffs = $derived($gitStore.allDiffs);
  const isLoading = $derived($gitStore.isDiffsLoading);
  const changeCount = $derived($gitStore.repoInfo?.statuses.length ?? 0);

  // Track which sections are visible and should render their content
  const visibleSections = new SvelteSet<string>();

  // Track visible file headers for active state
  const visibleHeaders = new SvelteMap<string, number>();

  interface DiffLine {
    type: 'add' | 'remove' | 'context' | 'header';
    content: string;
    lineNumber: number | null;
  }

  // Cache parsed diffs to avoid re-parsing
  const parsedDiffCache = new SvelteMap<string, DiffLine[]>();

  function parseDiff(path: string, diffContent: string): DiffLine[] {
    if (parsedDiffCache.has(path)) {
      return parsedDiffCache.get(path)!;
    }

    if (!diffContent) {
      parsedDiffCache.set(path, []);
      return [];
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

  function getDiffId(path: string): string {
    return `diff-${path.replace(/[^a-zA-Z0-9]/g, '-')}`;
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
            visibleSections.add(path);
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

  // Update store with the topmost visible file
  $effect(() => {
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
  });

  // Clear on unmount
  onDestroy(() => {
    gitStore.setCurrentVisibleFile(null);
  });

  // Clear cache when diffs change
  $effect(() => {
    if (allDiffs) {
      parsedDiffCache.clear();
      visibleSections.clear();
    }
  });
</script>

<div class="diff-view">
  <div class="diff-header">
    <div class="header-title">
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
        <circle cx="12" cy="12" r="10"></circle>
        <path d="M12 6v6l4 2"></path>
      </svg>
      <span>CHANGES</span>
      {#if changeCount > 0}
        <span class="badge">{changeCount}</span>
      {/if}
    </div>
  </div>

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
            {:else}
              <!-- Placeholder while not visible -->
              <div
                class="diff-placeholder"
                style="height: {estimateLineCount(fileDiff.diff) * 22}px"
              >
                <span class="placeholder-text">Scroll to load diff...</span>
              </div>
            {/if}
          </div>
        </div>
      {/each}
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

  .diff-header {
    display: flex;
    align-items: center;
    height: var(--tabbar-height, 44px);
    padding: 0 var(--space-4);
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.08em;
  }

  .header-title svg {
    color: var(--git-modified);
    opacity: 0.8;
  }

  .badge {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    background: var(--git-modified);
    color: var(--bg-primary);
    font-size: 10px;
    font-weight: 700;
    border-radius: 9px;
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
