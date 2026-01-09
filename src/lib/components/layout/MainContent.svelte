<script lang="ts">
  import type { ViewMode } from '@/lib/stores/appStore';
  import { Terminal } from '@/lib/components/terminal';
  import { Editor } from '@/lib/components/editor';

  interface Props {
    mode?: ViewMode;
    currentFile?: string | null;
    onModeToggle?: () => void;
  }

  let { mode = 'terminal', currentFile = null, onModeToggle }: Props = $props();
</script>

<main class="main-content">
  <div class="content-header">
    <div class="header-left">
      <button
        class="mode-tab"
        class:active={mode === 'terminal'}
        onclick={() => mode !== 'terminal' && onModeToggle?.()}
      >
        ⌨ Terminal
      </button>
      {#if currentFile}
        <button
          class="mode-tab"
          class:active={mode === 'editor'}
          onclick={() => mode !== 'editor' && onModeToggle?.()}
        >
          📄 {currentFile.split('/').pop()}
        </button>
      {/if}
    </div>
  </div>
  <div class="content-area">
    {#if mode === 'terminal'}
      <Terminal />
    {:else}
      <Editor filePath={currentFile} />
    {/if}
  </div>
</main>

<style>
  .main-content {
    height: 100%;
    background-color: var(--bg-primary);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .content-header {
    height: 35px;
    padding: 0;
    display: flex;
    align-items: stretch;
    background-color: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-color);
  }

  .header-left {
    display: flex;
    align-items: stretch;
  }

  .mode-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 16px;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition:
      background-color 0.15s,
      color 0.15s;
  }

  .mode-tab:hover {
    background-color: var(--bg-secondary);
    color: var(--text-primary);
  }

  .mode-tab.active {
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border-bottom-color: var(--accent-color);
  }

  .content-area {
    flex: 1;
    overflow: hidden;
  }
</style>
