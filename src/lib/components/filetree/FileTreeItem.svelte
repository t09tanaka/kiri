<script lang="ts">
  import { fileService } from '@/lib/services/fileService';
  import type { FileEntry } from './types';
  import {
    type GitFileStatus,
    getStatusIcon,
    getStatusColor,
    getDirectoryStatusColor,
  } from '@/lib/stores/gitStore';
  import { getFileIconInfo, getFolderColor } from '@/lib/utils/fileIcons';
  import ContextMenu, { type MenuItem } from '@/lib/components/ui/ContextMenu.svelte';
  import { dragDropStore, isDragging, dropTargetPath } from '@/lib/stores/dragDropStore';
  import FileTreeItem from './FileTreeItem.svelte';

  interface Props {
    entry: FileEntry;
    depth?: number;
    selectedPath?: string | null;
    onSelect?: (path: string) => void;
    onOpenInTerminal?: (path: string) => void;
    gitStatusMap?: Map<string, GitFileStatus>;
    repoRoot?: string;
  }

  let {
    entry,
    depth = 0,
    selectedPath = null,
    onSelect,
    onOpenInTerminal,
    gitStatusMap = new Map(),
    repoRoot = '',
  }: Props = $props();

  let expanded = $state(false);
  let children = $state<FileEntry[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let contextMenu = $state<{ x: number; y: number } | null>(null);
  let isDeleted = $state(false);

  const isSelected = $derived(selectedPath === entry.path);
  const paddingLeft = $derived(12 + depth * 16);

  // File icon info
  const fileIconInfo = $derived(getFileIconInfo(entry.name));
  const folderColor = $derived(getFolderColor(expanded));

  const relativePath = $derived(() => {
    if (!repoRoot || !entry.path.startsWith(repoRoot)) return '';
    return entry.path.slice(repoRoot.length + 1);
  });

  // Path with leading slash for display/copy (indicates project root)
  const displayPath = $derived(() => {
    const rel = relativePath();
    return rel ? '/' + rel : '';
  });

  const gitStatus = $derived(() => {
    const path = relativePath();
    if (!path) return null;
    return gitStatusMap.get(path) ?? null;
  });

  const statusIcon = $derived(gitStatus() ? getStatusIcon(gitStatus()!) : '');

  // For files, use direct status color. For directories, check children.
  const statusColor = $derived(() => {
    if (entry.is_dir) {
      const path = relativePath();
      return getDirectoryStatusColor(path, gitStatusMap);
    }
    return gitStatus() ? getStatusColor(gitStatus()!) : '';
  });

  // Drop target detection
  const isDropTarget = $derived($isDragging && entry.is_dir && $dropTargetPath === entry.path);

  async function toggleExpand() {
    if (!entry.is_dir) return;

    if (expanded) {
      expanded = false;
      return;
    }

    loading = true;
    error = null;

    try {
      children = await fileService.readDirectory(entry.path);
      expanded = true;
    } catch (e) {
      error = String(e);
      console.error('Failed to read directory:', e);
    } finally {
      loading = false;
    }
  }

  function handleClick() {
    if (entry.is_dir) {
      toggleExpand();
    } else {
      onSelect?.(entry.path);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleClick();
    }
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY };
  }

  function getContextMenuItems(): MenuItem[] {
    const items: MenuItem[] = [];

    if (entry.is_dir) {
      items.push(
        { id: 'expand', label: expanded ? 'Collapse' : 'Expand', icon: expanded ? '▼' : '▶' },
        { id: 'open-terminal', label: 'Open in Terminal', icon: '>' },
        { id: 'separator1', label: '', separator: true },
        { id: 'copy-path', label: 'Copy Path', shortcut: '⌘C' },
        { id: 'reveal', label: 'Reveal in Finder', shortcut: '⌘⇧R' },
        { id: 'separator2', label: '', separator: true },
        { id: 'delete', label: 'Delete', danger: true }
      );
    } else {
      items.push(
        { id: 'open', label: 'Open' },
        { id: 'separator1', label: '', separator: true },
        { id: 'copy-path', label: 'Copy Path', shortcut: '⌘C' },
        { id: 'reveal', label: 'Reveal in Finder', shortcut: '⌘⇧R' },
        { id: 'separator2', label: '', separator: true },
        { id: 'delete', label: 'Delete', danger: true }
      );
    }

    return items;
  }

  async function handleContextMenuSelect(id: string) {
    switch (id) {
      case 'expand':
        toggleExpand();
        break;
      case 'open':
        onSelect?.(entry.path);
        break;
      case 'open-terminal':
        onOpenInTerminal?.(entry.path);
        break;
      case 'copy-path':
        await navigator.clipboard.writeText(displayPath() || entry.path);
        break;
      case 'reveal':
        await fileService.revealInFinder(entry.path);
        break;
      case 'delete':
        await handleDelete();
        break;
    }
  }

  async function handleDelete() {
    // Hide from UI first (optimistic update)
    isDeleted = true;

    try {
      await fileService.deletePath(entry.path);
    } catch (e) {
      console.error('Failed to delete:', e);
      // Restore visibility on error
      isDeleted = false;
    }
  }

  // Drag and drop handlers
  function handleDragMouseEnter() {
    if (!$isDragging || !entry.is_dir) return;

    dragDropStore.setDropTarget(entry.path);

    // Start auto-expand timer if directory is collapsed
    if (!expanded) {
      dragDropStore.startHoverTimer(entry.path, () => {
        toggleExpand();
      });
    }
  }

  function handleDragMouseLeave() {
    if (!$isDragging || !entry.is_dir) return;

    dragDropStore.clearHoverTimer(entry.path);
  }
</script>

{#if !isDeleted}
  <div
    class="tree-item-container"
    role="treeitem"
    onmouseenter={handleDragMouseEnter}
    onmouseleave={handleDragMouseLeave}
  >
    <button
      class="tree-item"
      class:selected={isSelected}
      class:gitignored={entry.is_gitignored}
      class:directory={entry.is_dir}
      class:drop-target={isDropTarget}
      style="padding-left: {paddingLeft}px"
      onclick={handleClick}
      onkeydown={handleKeyDown}
      oncontextmenu={handleContextMenu}
      title={entry.path}
    >
      {#if entry.is_dir}
        <span class="chevron" class:expanded>
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <polyline points="9 18 15 12 9 6"></polyline>
          </svg>
        </span>
        <span class="icon folder" style="color: {folderColor}">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            {#if expanded}
              <path
                d="M19 20H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h6l2 2h6a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2z"
              />
            {:else}
              <path
                d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
              />
            {/if}
          </svg>
        </span>
      {:else}
        <span class="spacer"></span>
        <span class="icon file" style="color: {fileIconInfo.color}">
          <svg
            width="14"
            height="14"
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
      {/if}
      <span class="name" class:git-modified={statusColor()} style:color={statusColor() || null}
        >{entry.name}</span
      >
      {#if statusIcon}
        <span class="git-status" style="color: {statusColor()}">{statusIcon}</span>
      {/if}
      {#if loading}
        <span class="loading">
          <svg
            class="spinner"
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <circle cx="12" cy="12" r="10" stroke-opacity="0.25" />
            <path d="M12 2a10 10 0 0 1 10 10" stroke-linecap="round" />
          </svg>
        </span>
      {/if}
    </button>

    {#if expanded && children.length > 0}
      <div class="children">
        {#each children as child, index (child.path)}
          <div class="child-wrapper" style="--child-index: {index}">
            <FileTreeItem
              entry={child}
              depth={depth + 1}
              {selectedPath}
              {onSelect}
              {onOpenInTerminal}
              {gitStatusMap}
              {repoRoot}
            />
          </div>
        {/each}
      </div>
    {/if}

    {#if error}
      <div class="error" style="padding-left: {paddingLeft + 16}px">
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="8" x2="12" y2="12"></line>
          <line x1="12" y1="16" x2="12.01" y2="16"></line>
        </svg>
        Failed to load
      </div>
    {/if}
  </div>
{/if}

{#if contextMenu}
  <ContextMenu
    items={getContextMenuItems()}
    x={contextMenu.x}
    y={contextMenu.y}
    onSelect={handleContextMenuSelect}
    onClose={() => (contextMenu = null)}
  />
{/if}

<style>
  .tree-item-container {
    width: 100%;
  }

  .tree-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    height: 28px;
    padding-right: var(--space-3);
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-sans);
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    border-radius: var(--radius-sm);
    margin: 1px var(--space-1);
    transition: all var(--transition-fast);
  }

  .tree-item:hover {
    background: var(--bg-tertiary);
  }

  .tree-item:hover .name:not(.git-modified) {
    color: var(--accent-color);
  }

  .tree-item:active {
    transform: scale(0.995);
    transition: transform 80ms ease;
  }

  .tree-item.selected {
    background: var(--accent-subtle);
    color: var(--text-primary);
  }

  .tree-item.selected::before {
    content: '';
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 2px;
    background: var(--accent-color);
    border-radius: 1px;
  }

  .tree-item.selected .name:not(.git-modified) {
    color: var(--accent-color);
  }

  /* Selection animation */
  .tree-item.selected {
    animation: selectFlash 0.4s ease;
  }

  @keyframes selectFlash {
    0% {
      background: rgba(125, 211, 252, 0.12);
    }
    100% {
      background: var(--accent-subtle);
    }
  }

  .tree-item.selected .icon {
    color: var(--accent-color);
  }

  .tree-item.gitignored {
    opacity: 0.4;
  }

  .chevron {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    color: var(--text-muted);
    transition:
      transform var(--transition-fast),
      color var(--transition-fast);
  }

  .tree-item:hover .chevron {
    color: var(--accent-color);
    transform: translateX(2px);
  }

  .chevron.expanded {
    transform: rotate(90deg);
    color: var(--accent-color);
  }

  .tree-item:hover .chevron.expanded {
    transform: rotate(90deg) translateX(2px);
  }

  .spacer {
    width: 16px;
    flex-shrink: 0;
  }

  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    transition: all var(--transition-fast);
  }

  .tree-item:hover .icon {
    color: var(--accent-color);
    transform: scale(1.1);
  }

  /* Hover mist effect */
  .tree-item::after {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(
      circle at var(--mouse-x, 50%) var(--mouse-y, 50%),
      rgba(125, 211, 252, 0.06) 0%,
      transparent 60%
    );
    opacity: 0;
    transition: opacity var(--transition-fast);
    pointer-events: none;
    border-radius: inherit;
  }

  .tree-item:hover::after {
    opacity: 1;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .directory .name {
    font-weight: 500;
  }

  .loading {
    display: flex;
    align-items: center;
    margin-left: auto;
  }

  .spinner {
    animation: spin 1s linear infinite;
    color: var(--accent-color);
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .git-status {
    flex-shrink: 0;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.05em;
    margin-left: auto;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid currentColor;
    background: transparent;
    opacity: 0.9;
    transition: all var(--transition-fast);
  }

  .tree-item:hover .git-status {
    opacity: 1;
    transform: scale(1.05);
  }

  .git-status {
    transform-origin: center;
  }

  .children {
    position: relative;
    width: 100%;
    animation: expandIn 0.2s ease-out;
    transform-origin: top;
  }

  /* Subtle vertical connection line */
  .children::before {
    content: '';
    position: absolute;
    left: calc(12px + var(--depth, 0) * 16px + 8px);
    top: 0;
    bottom: 0;
    width: 1px;
    background: linear-gradient(to bottom, var(--border-subtle), transparent);
    opacity: 0.3;
    pointer-events: none;
  }

  .child-wrapper {
    animation: childSlideIn 0.2s ease-out backwards;
    animation-delay: calc(var(--child-index) * 30ms);
  }

  @keyframes childSlideIn {
    from {
      opacity: 0;
      transform: translateX(-8px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  @keyframes expandIn {
    from {
      opacity: 0;
      transform: scaleY(0.95);
    }
    to {
      opacity: 1;
      transform: scaleY(1);
    }
  }

  .error {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--git-deleted);
    font-size: 11px;
    padding: var(--space-2) var(--space-3);
    margin: var(--space-1);
    background: rgba(255, 69, 58, 0.08);
    border-radius: var(--radius-sm);
    border: 1px solid rgba(255, 69, 58, 0.15);
    animation: errorShake 0.4s ease;
  }

  .error svg {
    flex-shrink: 0;
  }

  @keyframes errorShake {
    0%,
    100% {
      transform: translateX(0);
    }
    20% {
      transform: translateX(-4px);
    }
    40% {
      transform: translateX(4px);
    }
    60% {
      transform: translateX(-2px);
    }
    80% {
      transform: translateX(2px);
    }
  }

  /* Drop target styles */
  .tree-item.drop-target {
    background: var(--accent-subtle);
    border: 1px solid var(--border-glow);
    box-shadow: 0 0 20px rgba(125, 211, 252, 0.1);
    animation: dropTargetGlow 1.5s ease-in-out infinite;
  }

  .tree-item.drop-target .icon {
    color: var(--accent-color);
    transform: scale(1.15);
  }

  .tree-item.drop-target .name {
    color: var(--accent-color);
  }

  @keyframes dropTargetGlow {
    0%,
    100% {
      box-shadow: 0 0 12px rgba(125, 211, 252, 0.1);
    }
    50% {
      box-shadow: 0 0 24px rgba(125, 211, 252, 0.2);
    }
  }
</style>
