<script lang="ts">
  import type { ViewMode } from '@/lib/stores/appStore';
  import { Terminal } from '@/lib/components/terminal';
  import { Editor } from '@/lib/components/editor';

  interface Props {
    mode?: ViewMode;
    currentFile?: string | null;
  }

  let { mode = 'terminal', currentFile = null }: Props = $props();
</script>

<main class="main-content">
  <div class="content-header">
    <span class="mode-indicator">{mode === 'terminal' ? 'TERMINAL' : 'EDITOR'}</span>
    {#if mode === 'editor' && currentFile}
      <span class="file-name">{currentFile.split('/').pop()}</span>
    {/if}
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
    padding: 0 12px;
    display: flex;
    align-items: center;
    gap: 12px;
    background-color: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .mode-indicator {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .file-name {
    font-size: 13px;
    color: var(--text-primary);
  }

  .content-area {
    flex: 1;
    overflow: hidden;
  }
</style>
