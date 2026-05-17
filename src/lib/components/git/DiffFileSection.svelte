<script lang="ts">
  import type { Action } from 'svelte/action';
  import { getStatusIcon, getStatusColor } from '@/lib/stores/gitStore';
  import { getFileIconInfo } from '@/lib/utils/fileIcons';
  import {
    estimateLineCount,
    getDiffId,
    getFileName,
    parseDiff,
    type DiffLine,
  } from './diffParser';
  import { createDiffCache } from '@/lib/utils/diffCache';
  import DiffImagePanel from './DiffImagePanel.svelte';

  interface FileDiff {
    path: string;
    status: string;
    diff: string;
    is_binary?: boolean;
    original_content_base64?: string | null;
    current_content_base64?: string | null;
  }

  interface Props {
    file: FileDiff;
    isVisible: boolean;
    lazyLoad: Action<HTMLElement, string>;
    trackHeader: Action<HTMLElement, string>;
  }

  let { file, isVisible, lazyLoad, trackHeader }: Props = $props();

  // Per-section cache so parsing happens once per file regardless of
  // how many times Svelte re-renders the section while scrolling.
  const linesCache = createDiffCache<DiffLine[]>();
  $effect(() => {
    void file.diff;
    linesCache.clear();
  });

  function linesFor(): DiffLine[] {
    return linesCache.getOrCompute(file.path, () => parseDiff(file.path, file.diff));
  }
</script>

<div class="file-section" id={getDiffId(file.path)} use:lazyLoad={file.path}>
  <div class="file-header" use:trackHeader={file.path}>
    <span class="status-badge" style="color: {getStatusColor(file.status)}">
      {getStatusIcon(file.status)}
    </span>
    <span class="file-name" style="color: {getFileIconInfo(getFileName(file.path)).color}"
      >{getFileName(file.path)}</span
    >
    <span class="file-path">{file.path}</span>
  </div>

  <div class="diff-content">
    {#if isVisible}
      {#if file.is_binary}
        <DiffImagePanel
          path={file.path}
          originalBase64={file.original_content_base64}
          currentBase64={file.current_content_base64}
        />
      {:else}
        {@const lines = linesFor()}
        {#if lines.length > 0}
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
        {:else}
          <div class="no-diff">
            <span>No visible changes</span>
          </div>
        {/if}
      {/if}
    {:else}
      <div
        class="diff-placeholder"
        style="height: {file.is_binary ? 200 : estimateLineCount(file.diff) * 22}px"
      >
        <span class="placeholder-text">Scroll to load diff...</span>
      </div>
    {/if}
  </div>
</div>

<style>
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

  .no-diff {
    padding: var(--space-4);
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
  }
</style>
