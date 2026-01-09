<script lang="ts">
  interface Props {
    width?: number;
  }

  let { width = 200 }: Props = $props();
</script>

<aside class="sidebar" style="width: {width}px">
  <div class="sidebar-header">
    <span class="title">EXPLORER</span>
  </div>
  <div class="sidebar-content">
    <p class="placeholder">File tree will be here</p>
  </div>
</aside>

<style>
  .sidebar {
    height: 100%;
    background-color: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    height: 35px;
    padding: 0 12px;
    display: flex;
    align-items: center;
    text-transform: uppercase;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .placeholder {
    padding: 8px 12px;
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
