<script lang="ts">
  import type { ViewMode } from '@/lib/stores/appStore';

  interface Props {
    mode?: ViewMode;
    currentFile?: string | null;
    onModeToggle?: () => void;
  }

  let { mode = 'terminal', currentFile = null, onModeToggle }: Props = $props();

  function handleModeClick() {
    onModeToggle?.();
  }
</script>

<footer class="status-bar">
  <div class="status-left">
    <button class="status-item mode-toggle" onclick={handleModeClick} title="Click to toggle mode">
      {mode === 'terminal' ? '⌨ Terminal' : '📝 Editor'}
    </button>
  </div>
  <div class="status-right">
    {#if currentFile}
      <span class="status-item file-path" title={currentFile}>
        {currentFile.split('/').slice(-2).join('/')}
      </span>
    {/if}
    <span class="status-item">Kiri</span>
  </div>
</footer>

<style>
  .status-bar {
    height: var(--statusbar-height);
    background-color: var(--accent-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px;
    font-size: 12px;
    color: white;
  }

  .status-left,
  .status-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .status-item {
    display: flex;
    align-items: center;
  }

  .mode-toggle {
    background: none;
    border: none;
    color: white;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 3px;
    transition: background-color 0.15s;
  }

  .mode-toggle:hover {
    background-color: rgba(255, 255, 255, 0.2);
  }

  .file-path {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
