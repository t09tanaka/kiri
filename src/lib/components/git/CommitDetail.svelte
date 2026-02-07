<script lang="ts">
  import type { CommitDiffResult } from '@/lib/services/gitService';
  import {
    detectEmbeddedContext,
    escapeHtml,
    getLanguageFromPath,
    getLineLanguage,
    highlightLine,
    supportsEmbeddedLanguages,
    type EmbeddedContext,
  } from '@/lib/utils/syntaxHighlight';
  import { Spinner } from '@/lib/components/ui';

  interface Props {
    diff: CommitDiffResult | null;
    isLoading: boolean;
  }

  let { diff, isLoading }: Props = $props();

  // Track which files are expanded
  let expandedFiles = $state(new Set<string>());

  function toggleFile(path: string) {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- immutable update pattern
    const next = new Set(expandedFiles);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    expandedFiles = next;
  }

  // Auto-expand all files when diff changes
  $effect(() => {
    if (diff && diff.files.length > 0) {
      expandedFiles = new Set(diff.files.map((f) => f.path));
    }
  });

  function getStatusIcon(status: string): string {
    switch (status) {
      case 'Added':
        return 'A';
      case 'Modified':
        return 'M';
      case 'Deleted':
        return 'D';
      case 'Renamed':
        return 'R';
      default:
        return '?';
    }
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'Added':
        return '#4ade80';
      case 'Modified':
        return '#fbbf24';
      case 'Deleted':
        return '#f87171';
      case 'Renamed':
        return '#7dd3fc';
      default:
        return 'var(--text-muted)';
    }
  }

  function formatDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  function getFileName(path: string): string {
    return path.split('/').pop() ?? path;
  }

  // DiffLine interface matching DiffView.svelte pattern
  interface DiffLine {
    type: 'add' | 'remove' | 'context' | 'header';
    content: string;
    highlightedContent: string;
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

    const baseLanguage = getLanguageFromPath(path);
    const hasEmbedded = supportsEmbeddedLanguages(path);
    const lines: DiffLine[] = [];
    let lineNum = 0;
    let context: EmbeddedContext = 'template';

    const rawLines = diffContent.split('\n');

    for (let i = 0; i < rawLines.length; i++) {
      const line = rawLines[i];
      let content: string;
      let type: DiffLine['type'];
      let lineNumber: number | null;

      if (line.startsWith('+ ')) {
        lineNum++;
        content = line.slice(2);
        type = 'add';
        lineNumber = lineNum;
      } else if (line.startsWith('- ')) {
        content = line.slice(2);
        type = 'remove';
        lineNumber = null;
      } else if (line.startsWith('  ')) {
        lineNum++;
        content = line.slice(2);
        type = 'context';
        lineNumber = lineNum;
      } else if (line.startsWith('@@')) {
        const match = line.match(/@@ -\d+(?:,\d+)? \+(\d+)/);
        if (match) {
          lineNum = parseInt(match[1], 10) - 1;
        }

        // For embedded language files, detect the context for this hunk
        if (hasEmbedded) {
          const upcomingContent: string[] = [];
          for (let j = i + 1; j < rawLines.length; j++) {
            const next = rawLines[j];
            if (next.startsWith('@@')) break;
            if (next.startsWith('+ ') || next.startsWith('- ') || next.startsWith('  ')) {
              upcomingContent.push(next.slice(2));
            }
            if (upcomingContent.length >= 10) break;
          }
          context = detectEmbeddedContext(upcomingContent);
        }

        lines.push({
          type: 'header',
          content: line,
          highlightedContent: escapeHtml(line),
          lineNumber: null,
        });
        continue;
      } else {
        continue;
      }

      // Determine language for this line
      let language = baseLanguage;
      if (hasEmbedded) {
        const result = getLineLanguage(content, baseLanguage, context);
        language = result.language;
        context = result.newContext;
      }

      lines.push({
        type,
        content,
        highlightedContent: highlightLine(content, language),
        lineNumber,
      });
    }

    parsedDiffCache.set(path, lines);
    return lines;
  }

  // Clear cache when diff changes
  $effect(() => {
    if (diff) {
      parsedDiffCache.clear();
    }
  });
</script>

<div class="commit-detail">
  {#if isLoading}
    <div class="loading-state">
      <Spinner size="md" />
      <span class="loading-text">Loading diff...</span>
    </div>
  {:else if !diff}
    <div class="empty-state">
      <svg
        width="36"
        height="36"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <circle cx="12" cy="12" r="4"></circle>
        <line x1="1.05" y1="12" x2="7" y2="12"></line>
        <line x1="17.01" y1="12" x2="22.96" y2="12"></line>
      </svg>
      <span>Select a commit to view details</span>
    </div>
  {:else}
    <!-- Commit header -->
    <div class="commit-header">
      <div class="commit-message-primary">{diff.commit.message.split('\n')[0]}</div>
      {#if diff.commit.message_body.includes('\n')}
        <div class="commit-message-body">
          {diff.commit.message_body.split('\n').slice(1).join('\n').trim()}
        </div>
      {/if}
      <div class="commit-meta">
        <span class="meta-author">
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
            <circle cx="12" cy="7" r="4"></circle>
          </svg>
          {diff.commit.author}
          <span class="meta-email">&lt;{diff.commit.author_email}&gt;</span>
        </span>
        <span class="meta-date">{formatDate(diff.commit.date)}</span>
        <span class="meta-hash" class:unpushed={!diff.commit.is_pushed}>{diff.commit.full_hash.slice(0, 8)}</span>
      </div>
    </div>

    <!-- Summary bar -->
    <div class="summary-bar">
      <span class="summary-files"
        >{diff.files.length} file{diff.files.length !== 1 ? 's' : ''} changed</span
      >
      {#if diff.total_additions > 0}
        <span class="summary-add">+{diff.total_additions}</span>
      {/if}
      {#if diff.total_deletions > 0}
        <span class="summary-del">-{diff.total_deletions}</span>
      {/if}
    </div>

    <!-- File list -->
    <div class="file-list">
      {#each diff.files as file (file.path)}
        <div class="file-section">
          <button class="file-header" onclick={() => toggleFile(file.path)}>
            <span class="file-status" style="color: {getStatusColor(file.status)}">
              {getStatusIcon(file.status)}
            </span>
            <span class="file-name">{getFileName(file.path)}</span>
            <span class="file-path-detail">{file.path}</span>
            <span class="file-stats">
              {#if file.additions > 0}
                <span class="stat-add">+{file.additions}</span>
              {/if}
              {#if file.deletions > 0}
                <span class="stat-del">-{file.deletions}</span>
              {/if}
            </span>
            <svg
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              class="chevron"
              class:expanded={expandedFiles.has(file.path)}
            >
              <polyline points="9 18 15 12 9 6"></polyline>
            </svg>
          </button>

          {#if expandedFiles.has(file.path)}
            {@const lines = parseDiff(file.path, file.diff)}
            <div class="file-diff">
              {#each lines as line, index (index)}
                <div class="diff-line {line.type}">
                  <span class="line-number">
                    {line.lineNumber ?? ''}
                  </span>
                  <span class="line-prefix">
                    {#if line.type === 'add'}+{:else if line.type === 'remove'}-{:else if line.type === 'context'}&nbsp;{/if}
                  </span>
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -- content is sanitized by highlightLine() -->
                  <span class="line-content">{@html line.highlightedContent}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .commit-detail {
    height: 100%;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: rgba(125, 211, 252, 0.2) transparent;
  }

  .commit-detail::-webkit-scrollbar {
    width: 6px;
  }

  .commit-detail::-webkit-scrollbar-track {
    background: transparent;
  }

  .commit-detail::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.2);
    border-radius: 3px;
  }

  .commit-detail::-webkit-scrollbar-thumb:hover {
    background: rgba(125, 211, 252, 0.3);
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--space-3);
    color: var(--text-muted);
  }

  .loading-text {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--space-3);
    color: var(--text-muted);
    font-size: 13px;
  }

  .empty-state svg {
    opacity: 0.3;
  }

  /* Commit header */
  .commit-header {
    padding: var(--space-4);
    border-bottom: 1px solid var(--border-subtle);
  }

  .commit-message-primary {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.4;
  }

  .commit-message-body {
    font-size: 13px;
    color: var(--text-secondary);
    white-space: pre-wrap;
    margin-top: var(--space-2);
    line-height: 1.5;
  }

  .commit-meta {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-3);
    flex-wrap: wrap;
  }

  .meta-author {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .meta-author svg {
    color: var(--text-muted);
  }

  .meta-email {
    color: var(--text-muted);
    font-size: 11px;
  }

  .meta-date {
    font-size: 11px;
    color: var(--text-muted);
  }

  .meta-hash {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    padding: 2px 6px;
    background: var(--bg-elevated);
    border-radius: var(--radius-sm);
  }

  .meta-hash.unpushed {
    color: #fcd34d;
    background: rgba(252, 211, 77, 0.08);
  }

  /* Summary bar */
  .summary-bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    background: rgba(0, 0, 0, 0.15);
    border-bottom: 1px solid var(--border-subtle);
    font-size: 12px;
  }

  .summary-files {
    color: var(--text-secondary);
  }

  .summary-add {
    color: #4ade80;
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .summary-del {
    color: #f87171;
    font-weight: 600;
    font-family: var(--font-mono);
  }

  /* File list */
  .file-list {
    padding-bottom: var(--space-4);
  }

  .file-section {
    border-bottom: 1px solid var(--border-subtle);
  }

  .file-section:last-child {
    border-bottom: none;
  }

  .file-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-4);
    background: var(--bg-secondary);
    border: none;
    cursor: pointer;
    transition: background var(--transition-fast);
    text-align: left;
  }

  .file-header:hover {
    background: var(--bg-elevated);
  }

  .file-status {
    font-size: 10px;
    font-weight: 700;
    flex-shrink: 0;
    width: 16px;
    text-align: center;
  }

  .file-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    flex-shrink: 0;
  }

  .file-path-detail {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .file-stats {
    display: flex;
    gap: 4px;
    font-size: 10px;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .stat-add {
    color: #4ade80;
  }

  .stat-del {
    color: #f87171;
  }

  .chevron {
    flex-shrink: 0;
    color: var(--text-muted);
    transition: transform 0.2s ease;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  /* Diff lines */
  .file-diff {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
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

  /* Syntax highlighting (highlight.js) */
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
</style>
