<script lang="ts">
  import type { ViewMode } from '@/lib/stores/appStore';

  interface Props {
    mode?: ViewMode;
  }

  let { mode = 'terminal' }: Props = $props();
</script>

<main class="main-content">
  <div class="content-header">
    <span class="mode-indicator">{mode === 'terminal' ? 'TERMINAL' : 'EDITOR'}</span>
  </div>
  <div class="content-area">
    {#if mode === 'terminal'}
      <p class="placeholder">Terminal will be here</p>
    {:else}
      <p class="placeholder">Editor will be here</p>
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
    background-color: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .mode-indicator {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .content-area {
    flex: 1;
    overflow: auto;
    padding: 16px;
  }

  .placeholder {
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
