<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fileService } from '@/lib/services/fileService';
  import { EditorView, lineNumbers } from '@codemirror/view';
  import { EditorState, StateEffect, StateField, Compartment } from '@codemirror/state';
  import { Decoration, type DecorationSet } from '@codemirror/view';
  import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
  import { tags } from '@lezer/highlight';
  import { getLanguageExtension } from '../editor/languages';
  import { tabStore } from '@/lib/stores/tabStore';
  import { projectStore } from '@/lib/stores/projectStore';
  import { fontSize } from '@/lib/stores/settingsStore';
  import { Spinner } from '@/lib/components/ui';

  interface Props {
    filePath: string;
    lineNumber?: number;
    onClose: () => void;
  }

  let { filePath, lineNumber, onClose }: Props = $props();

  let editorContainer: HTMLDivElement | null = $state(null);
  let view: EditorView | null = null;
  let loading = $state(true);
  let error = $state<string | null>(null);
  let mounted = $state(false);
  let fileContent = $state<string | null>(null);

  // Compartment for dynamic theme updates (font size)
  const themeCompartment = new Compartment();

  // KIRI Mist theme colors - soft atmospheric palette (reused from Editor.svelte)
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
  ]);

  // Custom Ethereal Mist editor theme for peek (read-only)
  function createMistTheme(editorFontSize: number) {
    // Line number font size scales with editor font size
    const lineNumberFontSize = Math.max(9, Math.round(editorFontSize * 0.85));
    return EditorView.theme(
      {
        '&': {
          height: '100%',
          fontSize: `${editorFontSize}px`,
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
          borderLeftColor: 'transparent', // Hide cursor in read-only mode
        },
        '.cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection': {
          backgroundColor: `${mistColors.selection} !important`,
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
          fontSize: `${lineNumberFontSize}px`,
          fontFamily: "'JetBrains Mono', 'SF Mono', monospace",
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
      },
      { dark: true }
    );
  }

  // Line highlight effect and field
  const highlightLineEffect = StateEffect.define<number>();

  const highlightDecoration = Decoration.line({
    attributes: { class: 'cm-highlighted-line' },
  });

  const highlightField = StateField.define<DecorationSet>({
    create() {
      return Decoration.none;
    },
    update(decorations, tr) {
      for (const effect of tr.effects) {
        if (effect.is(highlightLineEffect)) {
          const lineNum = effect.value;
          if (lineNum > 0 && lineNum <= tr.state.doc.lines) {
            const line = tr.state.doc.line(lineNum);
            return Decoration.set([highlightDecoration.range(line.from)]);
          }
        }
      }
      return decorations;
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  function scrollToLine(editorView: EditorView, line: number) {
    if (line < 1 || line > editorView.state.doc.lines) return;

    const docLine = editorView.state.doc.line(line);

    editorView.dispatch({
      effects: [
        highlightLineEffect.of(line),
        EditorView.scrollIntoView(docLine.from, {
          y: 'center',
          yMargin: 100,
        }),
      ],
      selection: { anchor: docLine.from },
    });
  }

  /**
   * Resolve relative path to absolute path using project root
   */
  function resolveFilePath(path: string): string {
    // If already absolute, return as-is
    if (path.startsWith('/')) {
      return path;
    }

    // Get project root and resolve relative path
    const projectRoot = projectStore.getCurrentPath();
    if (projectRoot) {
      // Remove leading ./ if present
      const cleanPath = path.startsWith('./') ? path.slice(2) : path;
      return `${projectRoot}/${cleanPath}`;
    }

    // Fallback to original path
    return path;
  }

  async function loadFile() {
    loading = true;
    error = null;
    fileContent = null;

    try {
      const absolutePath = resolveFilePath(filePath);
      const content = await fileService.readFile(absolutePath);
      fileContent = content;
    } catch (e) {
      error = String(e);
      console.error('Failed to read file:', e);
    } finally {
      loading = false;
    }
  }

  // Create editor when container is available and content is loaded
  $effect(() => {
    if (editorContainer && fileContent !== null && !view) {
      createEditor(fileContent);
    }
  });

  function createEditor(content: string) {
    if (view) {
      view.destroy();
      view = null;
    }

    if (!editorContainer) {
      return;
    }

    // Get current font size from store
    const currentFontSize = $fontSize;

    const extensions = [
      lineNumbers(),
      EditorView.editable.of(false), // Read-only
      EditorView.contentAttributes.of({ tabindex: '0' }),
      syntaxHighlighting(mistHighlightStyle),
      themeCompartment.of(createMistTheme(currentFontSize)),
      highlightField,
    ];

    // Add language extension if available
    const langExt = getLanguageExtension(filePath);
    if (langExt) {
      extensions.push(langExt);
    }

    const state = EditorState.create({
      doc: content,
      extensions,
    });

    view = new EditorView({
      state,
      parent: editorContainer,
    });

    // Scroll to line after editor is created
    if (lineNumber && view) {
      requestAnimationFrame(() => {
        if (view) {
          scrollToLine(view, lineNumber);
        }
      });
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      handleOpenInEditor();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleOpenInEditor() {
    const absolutePath = resolveFilePath(filePath);
    tabStore.addEditorTab(absolutePath);
    onClose();
  }

  function getFileName(path: string): string {
    return path.split('/').pop() || path;
  }

  onMount(() => {
    mounted = true;
    loadFile();
    // Use capture phase to intercept before terminal handles it
    document.addEventListener('keydown', handleKeyDown, true);

    // Subscribe to font size changes and update editor theme
    const unsubscribe = fontSize.subscribe((size) => {
      if (view) {
        view.dispatch({
          effects: themeCompartment.reconfigure(createMistTheme(size)),
        });
      }
    });

    return () => {
      unsubscribe();
    };
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
    if (view) {
      view.destroy();
    }
  });
</script>

<div
  class="peek-backdrop"
  class:mounted
  onclick={handleBackdropClick}
  onkeydown={() => {}}
  role="button"
  tabindex="-1"
>
  <div class="peek-editor">
    <div class="modal-glow"></div>
    <div class="modal-content">
      <div class="peek-header">
        <div class="file-info">
          <span class="file-icon">
            <svg
              width="16"
              height="16"
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
          <span class="file-name">{getFileName(filePath)}</span>
          {#if lineNumber}
            <span class="line-number">:{lineNumber}</span>
          {/if}
          <span class="file-path">{filePath}</span>
        </div>
        <div class="header-actions">
          <button
            class="action-btn open-btn"
            onclick={handleOpenInEditor}
            title="Open in Editor"
            aria-label="Open in Editor"
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
          <button
            class="action-btn close-btn"
            onclick={onClose}
            title="Close (Esc)"
            aria-label="Close"
          >
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

      <div class="peek-body">
        {#if loading}
          <div class="loading">
            <Spinner size="lg" />
            <span class="loading-text">Loading file...</span>
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
        {:else if fileContent !== null}
          <div class="editor-container" bind:this={editorContainer}></div>
        {/if}
      </div>

      <div class="peek-footer">
        <span class="footer-item">
          <kbd>Esc</kbd>
          <span>close</span>
        </span>
        <span class="footer-item">
          <kbd>Enter</kbd>
          <span>open in editor</span>
        </span>
      </div>
    </div>
  </div>
</div>

<style>
  .peek-backdrop {
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

  .peek-backdrop.mounted {
    opacity: 1;
  }

  .peek-editor {
    position: relative;
    width: 80%;
    max-width: 900px;
    height: 60%;
    max-height: 600px;
    min-height: 300px;
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

  .peek-editor:hover .modal-glow {
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

  .peek-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid var(--border-color);
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    overflow: hidden;
  }

  .file-icon {
    flex-shrink: 0;
    color: var(--accent-color);
    display: flex;
    align-items: center;
  }

  .file-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .line-number {
    font-size: 14px;
    font-weight: 500;
    color: var(--accent-color);
    font-family: var(--font-mono);
  }

  .file-path {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    margin-left: var(--space-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
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

  .action-btn:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
  }

  .action-btn.close-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .peek-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .editor-container {
    width: 100%;
    height: 100%;
  }

  /* Highlighted line styling */
  .editor-container :global(.cm-highlighted-line) {
    background: rgba(125, 211, 252, 0.15) !important;
    position: relative;
  }

  .editor-container :global(.cm-highlighted-line)::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--accent-color);
  }

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

  .error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--space-4);
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
  }

  .peek-footer {
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
