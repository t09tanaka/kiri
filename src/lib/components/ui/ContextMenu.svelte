<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  export interface MenuItem {
    id: string;
    label: string;
    icon?: string;
    shortcut?: string;
    disabled?: boolean;
    separator?: boolean;
    danger?: boolean;
  }

  interface Props {
    items: MenuItem[];
    x: number;
    y: number;
    onSelect: (id: string) => void;
    onClose: () => void;
  }

  let { items, x, y, onSelect, onClose }: Props = $props();

  let menuRef: HTMLDivElement;
  let adjustedX = $state(x);
  let adjustedY = $state(y);
  let visible = $state(false);

  function handleClick(item: MenuItem) {
    if (item.disabled || item.separator) return;
    onSelect(item.id);
    onClose();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (menuRef && !menuRef.contains(e.target as Node)) {
      onClose();
    }
  }

  onMount(() => {
    // Adjust position to keep menu within viewport
    requestAnimationFrame(() => {
      if (menuRef) {
        const rect = menuRef.getBoundingClientRect();
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;

        if (x + rect.width > viewportWidth) {
          adjustedX = viewportWidth - rect.width - 8;
        }
        if (y + rect.height > viewportHeight) {
          adjustedY = viewportHeight - rect.height - 8;
        }

        visible = true;
      }
    });

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleKeyDown);
  });

  onDestroy(() => {
    document.removeEventListener('mousedown', handleClickOutside);
    document.removeEventListener('keydown', handleKeyDown);
  });
</script>

<div
  class="context-menu"
  class:visible
  style="left: {adjustedX}px; top: {adjustedY}px"
  bind:this={menuRef}
  role="menu"
>
  <div class="menu-glow"></div>
  {#each items as item, index (item.id)}
    {#if item.separator}
      <div class="separator"></div>
    {:else}
      <button
        class="menu-item"
        class:disabled={item.disabled}
        class:danger={item.danger}
        onclick={() => handleClick(item)}
        role="menuitem"
        disabled={item.disabled}
        style="--item-index: {index}"
      >
        {#if item.icon}
          <span class="icon">{item.icon}</span>
        {:else}
          <span class="icon-placeholder"></span>
        {/if}
        <span class="label">{item.label}</span>
        {#if item.shortcut}
          <span class="shortcut">{item.shortcut}</span>
        {/if}
      </button>
    {/if}
  {/each}
</div>

<style>
  .context-menu {
    position: fixed;
    z-index: 1000;
    min-width: 180px;
    max-width: 280px;
    background: var(--bg-glass);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    padding: var(--space-1);
    opacity: 0;
    transform: scale(0.95) translateY(-4px);
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    overflow: hidden;
  }

  .menu-glow {
    position: absolute;
    inset: -1px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: inherit;
    opacity: 0.05;
    filter: blur(5px);
    z-index: -1;
    transition: opacity 0.2s ease;
  }

  .context-menu:hover .menu-glow {
    opacity: 0.08;
  }

  .context-menu.visible {
    opacity: 1;
    transform: scale(1) translateY(0);
  }

  .menu-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-sans);
    text-align: left;
    cursor: pointer;
    transition: all var(--transition-fast);
    animation: menuItemFadeIn 0.2s ease backwards;
    animation-delay: calc(var(--item-index) * 30ms);
  }

  @keyframes menuItemFadeIn {
    from {
      opacity: 0;
      transform: translateX(-8px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .menu-item:hover:not(.disabled) {
    background: linear-gradient(90deg, var(--accent-subtle) 0%, transparent 100%);
  }

  .menu-item:hover:not(.disabled)::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 2px;
    height: 16px;
    background: var(--accent-color);
    border-radius: 1px;
  }

  .menu-item:active:not(.disabled) {
    background: rgba(125, 211, 252, 0.15);
    transform: scale(0.98);
  }

  .menu-item.disabled {
    color: var(--text-muted);
    cursor: not-allowed;
    opacity: 0.5;
  }

  .menu-item.danger {
    color: var(--git-deleted);
  }

  .menu-item.danger:hover:not(.disabled) {
    background: rgba(255, 69, 58, 0.15);
  }

  .menu-item.danger:hover:not(.disabled)::before {
    background: var(--git-deleted);
  }

  .icon {
    flex-shrink: 0;
    width: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    transition: all var(--transition-fast);
  }

  .menu-item:hover:not(.disabled):not(.danger) .icon {
    color: var(--accent-color);
    transform: scale(1.1);
  }

  .icon-placeholder {
    width: 16px;
    flex-shrink: 0;
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: all var(--transition-fast);
  }

  .menu-item:hover:not(.disabled):not(.danger) .label {
    color: var(--accent-color);
  }

  .shortcut {
    flex-shrink: 0;
    color: var(--text-muted);
    font-size: 10px;
    font-family: var(--font-mono);
    padding: 3px 6px;
    background: linear-gradient(180deg, var(--bg-tertiary) 0%, var(--bg-secondary) 100%);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    box-shadow: 0 1px 0 var(--bg-primary);
    transition: all var(--transition-fast);
  }

  .menu-item:hover:not(.disabled) .shortcut {
    color: var(--accent-color);
    border-color: rgba(125, 211, 252, 0.2);
    transform: translateY(-1px);
    box-shadow: 0 2px 0 var(--bg-primary);
  }

  .separator {
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--border-color), transparent);
    margin: var(--space-1) var(--space-2);
  }

  /* Hover ripple effect */
  .menu-item::after {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(
      circle at var(--mouse-x, 50%) var(--mouse-y, 50%),
      rgba(125, 211, 252, 0.1) 0%,
      transparent 60%
    );
    opacity: 0;
    transition: opacity var(--transition-fast);
    pointer-events: none;
    border-radius: inherit;
  }

  .menu-item:hover:not(.disabled)::after {
    opacity: 1;
  }

  /* Menu appear from cursor */
  .context-menu {
    transform-origin: top left;
  }

  /* Top border glow */
  .context-menu::before {
    content: '';
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent-color), transparent);
    opacity: 0.5;
    z-index: 1;
  }
</style>
