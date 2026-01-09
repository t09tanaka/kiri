<script lang="ts">
  import { FileTree } from '@/lib/components/filetree';

  interface Props {
    width?: number;
    rootPath?: string;
    onFileSelect?: (path: string) => void;
  }

  let { width = 200, rootPath = '', onFileSelect }: Props = $props();
</script>

<aside class="sidebar" style="width: {width}px">
  <div class="sidebar-header">
    <span class="title">EXPLORER</span>
  </div>
  <div class="sidebar-content">
    <FileTree {rootPath} {onFileSelect} />
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
    overflow: hidden;
  }
</style>
