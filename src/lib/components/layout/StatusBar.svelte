<script lang="ts">
  import type { ViewMode } from '@/lib/stores/appStore';

  interface Props {
    mode?: ViewMode;
    currentFile?: string | null;
  }

  let { mode = 'terminal', currentFile = null }: Props = $props();
</script>

<footer class="status-bar">
  <div class="status-left">
    <span class="status-item mode">{mode === 'terminal' ? 'Terminal' : 'Editor'}</span>
  </div>
  <div class="status-right">
    {#if currentFile}
      <span class="status-item">{currentFile}</span>
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

  .mode {
    font-weight: 500;
  }
</style>
