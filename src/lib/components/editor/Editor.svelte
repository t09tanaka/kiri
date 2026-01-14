<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fileService } from '@/lib/services/fileService';
  import { gitService } from '@/lib/services/gitService';
  import { projectStore } from '@/lib/stores/projectStore';
  import { gitDiffExtension, updateGitDiff } from './extensions';
  import {
    EditorView,
    keymap,
    lineNumbers,
    highlightActiveLine,
    drawSelection,
  } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { defaultKeymap, indentWithTab } from '@codemirror/commands';
  import { syntaxHighlighting, HighlightStyle, bracketMatching } from '@codemirror/language';
  import { tags } from '@lezer/highlight';
  import { getLanguageExtension } from './languages';
  import { Spinner } from '@/lib/components/ui';

  interface Props {
    filePath: string | null;
    onSave?: () => void;
    onModifiedChange?: (modified: boolean) => void;
  }

  let { filePath, onSave, onModifiedChange }: Props = $props();

  let editorContainer: HTMLDivElement = $state(null!);
  let view: EditorView | null = null;
  let loading = $state(true);
  let error = $state<string | null>(null);
  let modified = $state(false);
  let isFocused = $state(false);

  // KIRI Mist theme colors - soft atmospheric palette
  const mistColors = {
    bg: '#0a0c10',
    bgHighlight: '#0e1116',
    fg: '#c8d3e0',
    fgMuted: '#5c6b7a',
    accent: '#7dd3fc',
    accentLavender: '#c4b5fd',
    selection: 'rgba(125, 211, 252, 0.18)',
    cursor: '#7dd3fc',
    lineNumber: '#3d4854',
    lineNumberActive: '#7dd3fc',
    // Syntax - soft, muted colors harmonized with mist theme
    keyword: '#c4b5fd',
    string: '#4ade80',
    number: '#fbbf24',
    comment: '#5c6b7a',
    function: '#7dd3fc',
    variable: '#93c5fd',
    type: '#d8b4fe',
    property: '#f87171',
    operator: '#fdba74',
    punctuation: '#8b99a8',
    // Markdown-specific colors
    heading: '#7dd3fc',
    emphasis: '#c4b5fd',
    strong: '#f0abfc',
    link: '#4ade80',
    url: '#5c6b7a',
    code: '#fbbf24',
    quote: '#a5b4fc',
    list: '#fdba74',
  };

  // Custom Mist highlight style
  const mistHighlightStyle = HighlightStyle.define([
    { tag: tags.keyword, color: mistColors.keyword, fontWeight: '500' },
    { tag: tags.controlKeyword, color: mistColors.keyword, fontWeight: '500' },
    { tag: tags.moduleKeyword, color: mistColors.keyword, fontWeight: '500' },
    { tag: tags.operatorKeyword, color: mistColors.keyword },
    { tag: tags.string, color: mistColors.string },
    { tag: tags.regexp, color: mistColors.string },
    { tag: tags.number, color: mistColors.number },
    { tag: tags.bool, color: mistColors.number },
    { tag: tags.null, color: mistColors.number },
    { tag: tags.comment, color: mistColors.comment, fontStyle: 'italic' },
    { tag: tags.lineComment, color: mistColors.comment, fontStyle: 'italic' },
    { tag: tags.blockComment, color: mistColors.comment, fontStyle: 'italic' },
    { tag: tags.function(tags.variableName), color: mistColors.function },
    { tag: tags.function(tags.propertyName), color: mistColors.function },
    { tag: tags.variableName, color: mistColors.variable },
    { tag: tags.definition(tags.variableName), color: mistColors.variable },
    { tag: tags.typeName, color: mistColors.type },
    { tag: tags.className, color: mistColors.type },
    { tag: tags.propertyName, color: mistColors.property },
    { tag: tags.operator, color: mistColors.operator },
    { tag: tags.punctuation, color: mistColors.punctuation },
    { tag: tags.bracket, color: mistColors.punctuation },
    { tag: tags.angleBracket, color: mistColors.punctuation },
    { tag: tags.squareBracket, color: mistColors.punctuation },
    { tag: tags.paren, color: mistColors.punctuation },
    { tag: tags.brace, color: mistColors.punctuation },
    { tag: tags.tagName, color: mistColors.keyword },
    { tag: tags.attributeName, color: mistColors.variable },
    { tag: tags.attributeValue, color: mistColors.string },
    // Markdown-specific tags
    { tag: tags.heading, color: mistColors.heading, fontWeight: '600' },
    { tag: tags.heading1, color: mistColors.heading, fontWeight: '700', fontSize: '1.4em' },
    { tag: tags.heading2, color: mistColors.heading, fontWeight: '600', fontSize: '1.2em' },
    { tag: tags.heading3, color: mistColors.heading, fontWeight: '600', fontSize: '1.1em' },
    { tag: tags.emphasis, color: mistColors.emphasis, fontStyle: 'italic' },
    { tag: tags.strong, color: mistColors.strong, fontWeight: '600' },
    { tag: tags.strikethrough, textDecoration: 'line-through', color: mistColors.fgMuted },
    { tag: tags.link, color: mistColors.link, textDecoration: 'underline' },
    { tag: tags.url, color: mistColors.url },
    { tag: tags.monospace, color: mistColors.code, fontFamily: 'monospace' },
    { tag: tags.quote, color: mistColors.quote, fontStyle: 'italic' },
    { tag: tags.list, color: mistColors.list },
    { tag: tags.contentSeparator, color: mistColors.fgMuted },
    { tag: tags.processingInstruction, color: mistColors.fgMuted },
  ]);

  // Custom Ethereal Mist editor theme
  const mistTheme = EditorView.theme(
    {
      '&': {
        height: '100%',
        fontSize: '13px',
        backgroundColor: mistColors.bg,
        color: mistColors.fg,
      },
      '.cm-content': {
        padding: '16px 0',
        caretColor: mistColors.cursor,
        fontFamily: "'JetBrains Mono', 'SF Mono', 'Fira Code', monospace",
        lineHeight: '1.7',
        letterSpacing: '0.02em',
      },
      '.cm-cursor, .cm-dropCursor': {
        borderLeftColor: mistColors.cursor,
        borderLeftWidth: '2px',
      },
      '&.cm-focused .cm-cursor': {
        borderLeftColor: mistColors.cursor,
      },
      '.cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection': {
        backgroundColor: `${mistColors.selection} !important`,
      },
      '.cm-activeLine': {
        backgroundColor: 'rgba(125, 211, 252, 0.03)',
      },
      '.cm-activeLineGutter': {
        backgroundColor: 'rgba(125, 211, 252, 0.03)',
      },
      '.cm-gutters': {
        backgroundColor: mistColors.bg,
        borderRight: '1px solid rgba(125, 211, 252, 0.08)',
        color: mistColors.lineNumber,
        paddingRight: '8px',
      },
      '.cm-lineNumbers .cm-gutterElement': {
        minWidth: '48px',
        padding: '0 12px 0 8px',
        fontSize: '11px',
        fontFamily: "'JetBrains Mono', 'SF Mono', monospace",
      },
      '.cm-lineNumbers .cm-gutterElement.cm-activeLineGutter': {
        color: mistColors.lineNumberActive,
        fontWeight: '500',
      },
      '.cm-foldGutter': {
        width: '14px',
      },
      '.cm-matchingBracket': {
        backgroundColor: 'rgba(125, 211, 252, 0.15)',
        outline: '1px solid rgba(125, 211, 252, 0.5)',
        borderRadius: '2px',
      },
      '.cm-nonmatchingBracket': {
        backgroundColor: 'rgba(255, 69, 58, 0.2)',
        outline: '1px solid rgba(255, 69, 58, 0.5)',
      },
      '.cm-scroller': {
        fontFamily: "'JetBrains Mono', 'SF Mono', 'Fira Code', monospace",
        overflow: 'auto',
      },
      '.cm-scroller::-webkit-scrollbar': {
        width: '8px',
        height: '8px',
      },
      '.cm-scroller::-webkit-scrollbar-track': {
        background: 'transparent',
      },
      '.cm-scroller::-webkit-scrollbar-thumb': {
        background: 'rgba(125, 211, 252, 0.12)',
        borderRadius: '4px',
      },
      '.cm-scroller::-webkit-scrollbar-thumb:hover': {
        background: 'rgba(125, 211, 252, 0.2)',
      },
      '.cm-tooltip': {
        backgroundColor: 'rgba(15, 20, 25, 0.95)',
        backdropFilter: 'blur(12px)',
        border: '1px solid rgba(125, 211, 252, 0.15)',
        borderRadius: '8px',
        boxShadow: '0 8px 32px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(125, 211, 252, 0.1)',
      },
      '.cm-tooltip-autocomplete': {
        '& > ul': {
          fontFamily: "'JetBrains Mono', 'SF Mono', monospace",
          fontSize: '12px',
        },
        '& > ul > li': {
          padding: '6px 12px',
          borderRadius: '4px',
          margin: '2px 4px',
        },
        '& > ul > li[aria-selected]': {
          backgroundColor: 'rgba(125, 211, 252, 0.15)',
          color: mistColors.accent,
        },
      },
    },
    { dark: true }
  );

  function setModified(value: boolean) {
    if (modified !== value) {
      modified = value;
      onModifiedChange?.(value);
    }
  }

  async function loadFile() {
    if (!filePath) {
      loading = false;
      return;
    }

    loading = true;
    error = null;
    setModified(false);

    try {
      const content = await fileService.readFile(filePath);
      createEditor(content);
      // Load git diff after editor is created (fire-and-forget to avoid blocking)
      loadGitDiff();
    } catch (e) {
      error = String(e);
      console.error('Failed to read file:', e);
    } finally {
      loading = false;
    }
  }

  async function loadGitDiff() {
    if (!filePath || !view) return;

    const repoPath = projectStore.getCurrentPath();
    if (!repoPath) return;

    try {
      // Convert absolute file path to relative path
      const relativePath = filePath.startsWith(repoPath + '/')
        ? filePath.slice(repoPath.length + 1)
        : filePath;

      const diff = await gitService.getFileDiff(repoPath, relativePath);
      // Only update if view still exists (might have been destroyed during await)
      if (view) {
        updateGitDiff(view, diff);
      }
    } catch {
      // Not a git repo or file not tracked - silently ignore
    }
  }

  async function saveFile() {
    if (!filePath || !view) return;

    try {
      const content = view.state.doc.toString();
      await fileService.writeFile(filePath, content);
      setModified(false);
      onSave?.();
      // Refresh git diff after save
      await loadGitDiff();
    } catch (e) {
      console.error('Failed to save file:', e);
      error = String(e);
    }
  }

  function createEditor(content: string) {
    if (view) {
      view.destroy();
    }

    const extensions = [
      lineNumbers(),
      highlightActiveLine(),
      bracketMatching(),
      drawSelection(),
      syntaxHighlighting(mistHighlightStyle),
      mistTheme,
      ...gitDiffExtension(),
      keymap.of([
        ...defaultKeymap,
        indentWithTab,
        {
          key: 'Mod-s',
          run: () => {
            saveFile();
            return true;
          },
        },
      ]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          setModified(true);
        }
      }),
    ];

    // Add language extension if available
    if (filePath) {
      const langExt = getLanguageExtension(filePath);
      if (langExt) {
        extensions.push(langExt);
      }
    }

    const state = EditorState.create({
      doc: content,
      extensions,
    });

    view = new EditorView({
      state,
      parent: editorContainer,
    });

    // Track focus state for visual feedback
    view.contentDOM.addEventListener('focus', () => {
      isFocused = true;
    });
    view.contentDOM.addEventListener('blur', () => {
      isFocused = false;
    });
  }

  onMount(() => {
    loadFile();
  });

  onDestroy(() => {
    if (view) {
      view.destroy();
    }
  });

  // Reload when filePath changes
  $effect(() => {
    if (filePath !== undefined) {
      loadFile();
    }
  });
</script>

<div class="editor-wrapper" class:focused={isFocused}>
  <div class="editor-glow"></div>
  <div class="focus-indicator"></div>
  <div class="scanlines"></div>
  <div class="code-reflection"></div>

  {#if loading}
    <div class="loading">
      <Spinner size="lg" />
      <span class="loading-text">Loading file...</span>
      <div class="loading-dots">
        <span class="dot"></span>
        <span class="dot"></span>
        <span class="dot"></span>
      </div>
    </div>
  {:else if error}
    <div class="error">
      <div class="error-icon">
        <svg
          width="28"
          height="28"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="8" x2="12" y2="12"></line>
          <line x1="12" y1="16" x2="12.01" y2="16"></line>
        </svg>
      </div>
      <span class="error-text">{error}</span>
    </div>
  {:else if !filePath}
    <div class="no-file">
      <div class="no-file-icon">
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
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
          <polyline points="14 2 14 8 20 8"></polyline>
        </svg>
      </div>
      <span class="no-file-text">Select a file to edit</span>
      <span class="no-file-hint">Choose from the file tree or use <kbd>⌘P</kbd></span>
    </div>
  {:else}
    <div class="editor-container" bind:this={editorContainer}></div>
  {/if}
</div>

<style>
  .editor-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
    background: linear-gradient(180deg, #060810 0%, #080b10 100%);
    overflow: hidden;
    animation: editorFadeIn 0.4s ease-out;
  }

  @keyframes editorFadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  /* Ambient corner glow */
  .editor-wrapper::before {
    content: '';
    position: absolute;
    bottom: 0;
    right: 0;
    width: 250px;
    height: 250px;
    background: radial-gradient(
      circle at bottom right,
      rgba(196, 181, 253, 0.025) 0%,
      transparent 70%
    );
    pointer-events: none;
  }

  .editor-wrapper::after {
    content: '';
    position: absolute;
    top: 0;
    left: 60px;
    width: 200px;
    height: 200px;
    background: radial-gradient(circle at top left, rgba(125, 211, 252, 0.02) 0%, transparent 70%);
    pointer-events: none;
  }

  .editor-container {
    position: relative;
    width: 100%;
    height: 100%;
    z-index: 1;
  }

  /* Atmospheric glow effect at the top */
  .editor-glow {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(125, 211, 252, 0.3) 50%,
      transparent 100%
    );
    opacity: 0.4;
    pointer-events: none;
    transition: all var(--transition-normal);
    z-index: 10;
  }

  .editor-wrapper.focused .editor-glow {
    opacity: 0.8;
    height: 2px;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(125, 211, 252, 0.5) 30%,
      rgba(196, 181, 253, 0.5) 70%,
      transparent 100%
    );
  }

  /* Focus indicator - gradient left border */
  .focus-indicator {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 2px;
    background: linear-gradient(180deg, var(--gradient-start), var(--gradient-end));
    opacity: 0;
    transition: opacity var(--transition-normal);
    pointer-events: none;
    z-index: 10;
  }

  .editor-wrapper.focused .focus-indicator {
    opacity: 1;
  }

  /* Loading state */
  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--space-4);
    color: var(--text-muted);
  }

  .loading-text {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .loading-dots {
    display: flex;
    gap: 6px;
  }

  .loading-dots .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-color);
    animation: loadingDot 1.4s ease-in-out infinite;
  }

  .loading-dots .dot:nth-child(1) {
    animation-delay: 0s;
  }

  .loading-dots .dot:nth-child(2) {
    animation-delay: 0.2s;
  }

  .loading-dots .dot:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes loadingDot {
    0%,
    80%,
    100% {
      opacity: 0.3;
      transform: scale(0.8);
    }
    40% {
      opacity: 1;
      transform: scale(1);
    }
  }

  /* Error state */
  .error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--space-4);
    animation: errorShake 0.4s ease;
  }

  @keyframes errorShake {
    0%,
    100% {
      transform: translateX(0);
    }
    20% {
      transform: translateX(-6px);
    }
    40% {
      transform: translateX(6px);
    }
    60% {
      transform: translateX(-3px);
    }
    80% {
      transform: translateX(3px);
    }
  }

  .error-icon {
    color: var(--git-deleted);
    opacity: 0.7;
  }

  .error-text {
    font-size: 13px;
    color: var(--git-deleted);
    max-width: 300px;
    text-align: center;
    line-height: 1.5;
    animation: fadeIn 0.5s ease 0.3s backwards;
  }

  /* No file state */
  .no-file {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--space-3);
    animation: fadeIn 0.5s ease;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .no-file-icon {
    color: var(--accent-color);
    opacity: 0.3;
    margin-bottom: var(--space-2);
  }

  .no-file-text {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .no-file-hint {
    font-size: 12px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .no-file-hint kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 20px;
    padding: 0 6px;
    background: linear-gradient(180deg, var(--bg-tertiary) 0%, var(--bg-secondary) 100%);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-size: 10px;
    font-family: var(--font-mono);
    font-weight: 500;
    color: var(--accent-color);
    box-shadow: 0 2px 0 var(--bg-primary);
    transition: all var(--transition-fast);
  }

  .no-file-hint:hover kbd {
    border-color: var(--accent-color);
    box-shadow: 0 3px 0 var(--bg-primary);
    transform: translateY(-1px);
  }

  /* Scanline overlay effect */
  .scanlines {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: repeating-linear-gradient(
      0deg,
      transparent,
      transparent 2px,
      rgba(0, 0, 0, 0.02) 2px,
      rgba(0, 0, 0, 0.02) 4px
    );
    z-index: 20;
    opacity: 0.4;
  }

  .editor-wrapper.focused .scanlines {
    opacity: 0.3;
  }

  /* Code reflection effect at bottom */
  .code-reflection {
    position: absolute;
    bottom: 0;
    left: 60px;
    right: 0;
    height: 60px;
    background: linear-gradient(
      180deg,
      transparent 0%,
      rgba(125, 211, 252, 0.01) 50%,
      rgba(125, 211, 252, 0.02) 100%
    );
    pointer-events: none;
    z-index: 3;
    opacity: 0;
    transition: opacity 0.5s ease;
  }

  .editor-wrapper.focused .code-reflection {
    opacity: 1;
  }

  /* Active line enhanced glow */
  .editor-wrapper.focused :global(.cm-activeLine) {
    background-color: rgba(125, 211, 252, 0.04) !important;
    transition: background-color 0.2s ease;
  }

  .editor-wrapper.focused :global(.cm-activeLineGutter) {
    background-color: rgba(125, 211, 252, 0.04) !important;
  }

  /* Enhanced scrollbar on focus */
  .editor-wrapper.focused :global(.cm-scroller::-webkit-scrollbar-thumb) {
    background: rgba(125, 211, 252, 0.15);
  }

  .editor-wrapper.focused :global(.cm-scroller::-webkit-scrollbar-thumb:hover) {
    background: rgba(125, 211, 252, 0.25);
  }
</style>
