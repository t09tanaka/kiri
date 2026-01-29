<script lang="ts">
  import { FileTree } from '@/lib/components/filetree';

  interface Props {
    width?: number;
    rootPath?: string;
    onFileSelect?: (path: string) => void;
  }

  let { width = 250, rootPath = '', onFileSelect }: Props = $props();
</script>

<aside class="sidebar" data-testid="sidebar" style="width: {width}px">
  <div class="sidebar-header">
    <div class="header-icon">
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      </svg>
    </div>
    <span class="title">Explorer</span>
  </div>

  <div class="sidebar-content">
    <FileTree {rootPath} {onFileSelect} />
  </div>
</aside>

<style>
  .sidebar {
    height: 100%;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
  }

  /* Subtle gradient overlay */
  .sidebar::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(125, 211, 252, 0.015) 0%, transparent 40%);
    pointer-events: none;
    z-index: 0;
  }

  /* Right edge glow line */
  .sidebar::after {
    content: '';
    position: absolute;
    top: 0;
    right: 0;
    width: 1px;
    height: 100%;
    background: linear-gradient(
      180deg,
      rgba(125, 211, 252, 0.1) 0%,
      rgba(125, 211, 252, 0.03) 50%,
      transparent 100%
    );
    z-index: 2;
  }

  .sidebar-header {
    position: relative;
    height: var(--tabbar-height, 44px);
    padding: 0 var(--space-4);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    overflow: hidden;
  }

  /* Header subtle shimmer */
  .sidebar-header::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(125, 211, 252, 0.03) 50%,
      transparent 100%
    );
    transform: translateX(-100%);
    animation: headerShimmer 8s ease-in-out infinite;
  }

  @keyframes headerShimmer {
    0%,
    100% {
      transform: translateX(-100%);
    }
    50% {
      transform: translateX(100%);
    }
  }

  .header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-color);
    opacity: 0.7;
    transition: all var(--transition-normal);
  }

  .sidebar-header:hover .header-icon {
    opacity: 1;
    transform: scale(1.1);
  }

  .title {
    flex: 1;
    transition: color var(--transition-fast);
  }

  .sidebar-header:hover .title {
    color: var(--text-secondary);
  }

  .sidebar-content {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  /* Bottom fade gradient for overflow hint */
  .sidebar-content::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 40px;
    background: linear-gradient(180deg, transparent, var(--bg-secondary));
    pointer-events: none;
    opacity: 0.8;
    z-index: 1;
  }

  /* Scrollbar styling */
  .sidebar-content :global(::-webkit-scrollbar) {
    width: 6px;
  }

  .sidebar-content :global(::-webkit-scrollbar-track) {
    background: transparent;
  }

  .sidebar-content :global(::-webkit-scrollbar-thumb) {
    background: rgba(125, 211, 252, 0.1);
    border-radius: 3px;
    transition: background 0.2s ease;
  }

  .sidebar-content:hover :global(::-webkit-scrollbar-thumb) {
    background: rgba(125, 211, 252, 0.2);
  }

  .sidebar-content :global(::-webkit-scrollbar-thumb:hover) {
    background: rgba(125, 211, 252, 0.3);
  }
</style>
