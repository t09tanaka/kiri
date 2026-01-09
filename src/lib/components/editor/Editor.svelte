<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { EditorView, keymap, lineNumbers, highlightActiveLine } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { defaultKeymap, indentWithTab } from '@codemirror/commands';
  import { syntaxHighlighting, defaultHighlightStyle, bracketMatching } from '@codemirror/language';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { getLanguageExtension } from './languages';

  interface Props {
    filePath: string | null;
    onSave?: () => void;
  }

  let { filePath, onSave }: Props = $props();

  let editorContainer: HTMLDivElement;
  let view: EditorView | null = null;
  let loading = $state(true);
  let error = $state<string | null>(null);
  let modified = $state(false);

  async function loadFile() {
    if (!filePath) {
      loading = false;
      return;
    }

    loading = true;
    error = null;
    modified = false;

    try {
      const content = await invoke<string>('read_file', { path: filePath });
      createEditor(content);
    } catch (e) {
      error = String(e);
      console.error('Failed to read file:', e);
    } finally {
      loading = false;
    }
  }

  async function saveFile() {
    if (!filePath || !view) return;

    try {
      const content = view.state.doc.toString();
      await invoke('write_file', { path: filePath, content });
      modified = false;
      onSave?.();
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
      syntaxHighlighting(defaultHighlightStyle),
      oneDark,
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
          modified = true;
        }
      }),
      EditorView.theme({
        '&': {
          height: '100%',
          fontSize: '14px',
        },
        '.cm-scroller': {
          fontFamily: 'JetBrains Mono, Menlo, Monaco, monospace',
        },
        '.cm-content': {
          padding: '8px 0',
        },
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

<div class="editor-wrapper">
  {#if loading}
    <div class="loading">Loading...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else if !filePath}
    <div class="no-file">No file selected</div>
  {:else}
    {#if modified}
      <div class="modified-indicator">● Modified</div>
    {/if}
    <div class="editor-container" bind:this={editorContainer}></div>
  {/if}
</div>

<style>
  .editor-wrapper {
    width: 100%;
    height: 100%;
    position: relative;
    background-color: var(--bg-primary);
  }

  .editor-container {
    width: 100%;
    height: 100%;
  }

  .loading,
  .error,
  .no-file {
    padding: 16px;
    color: var(--text-secondary);
  }

  .error {
    color: #f44336;
  }

  .no-file {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    font-style: italic;
  }

  .modified-indicator {
    position: absolute;
    top: 8px;
    right: 16px;
    font-size: 12px;
    color: var(--accent-color);
    z-index: 10;
  }
</style>
