<script lang="ts">
  import { onDestroy } from 'svelte';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();

  let dialogRef = $state<HTMLDivElement | null>(null);
  let mounted = $state(false);

  interface ShortcutGroup {
    title: string;
    icon: string;
    shortcuts: Array<{
      keys: string[];
      description: string;
    }>;
  }

  const shortcutGroups: ShortcutGroup[] = [
    {
      title: 'General',
      icon: '⚡',
      shortcuts: [
        { keys: ['⌘', 'P'], description: 'Quick Open' },
        { keys: ['⌘', 'B'], description: 'Toggle Sidebar' },
        { keys: ['⌘', '⇧', 'N'], description: 'New Window' },
        { keys: ['?'], description: 'Show Shortcuts' },
      ],
    },
    {
      title: 'Tabs',
      icon: '📑',
      shortcuts: [
        { keys: ['⌘', '`'], description: 'New Terminal' },
        { keys: ['⌘', 'W'], description: 'Close Tab' },
        { keys: ['⌘', '1-9'], description: 'Switch to Tab' },
      ],
    },
    {
      title: 'Editor',
      icon: '📝',
      shortcuts: [
        { keys: ['⌘', 'S'], description: 'Save File' },
        { keys: ['⌘', 'Z'], description: 'Undo' },
        { keys: ['⌘', '⇧', 'Z'], description: 'Redo' },
      ],
    },
    {
      title: 'View',
      icon: '🔍',
      shortcuts: [
        { keys: ['⌘', '+'], description: 'Zoom In' },
        { keys: ['⌘', '-'], description: 'Zoom Out' },
        { keys: ['⌘', '0'], description: 'Reset Zoom' },
      ],
    },
    {
      title: 'File Tree',
      icon: '📁',
      shortcuts: [
        { keys: ['Enter'], description: 'Open / Expand' },
        { keys: ['Space'], description: 'Select' },
        { keys: ['Right Click'], description: 'Context Menu' },
      ],
    },
  ];

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === dialogRef) {
      onClose();
    }
  }

  $effect(() => {
    if (isOpen) {
      mounted = true;
      document.addEventListener('keydown', handleKeyDown);
    } else {
      document.removeEventListener('keydown', handleKeyDown);
      // Delay unmount for exit animation
      setTimeout(() => {
        if (!isOpen) mounted = false;
      }, 200);
    }
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown);
  });
</script>

{#if isOpen || mounted}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" class:visible={isOpen} bind:this={dialogRef} onclick={handleBackdropClick}>
    <div
      class="dialog"
      class:visible={isOpen}
      role="dialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
    >
      <div class="dialog-glow"></div>
      <div class="dialog-content">
        <div class="header">
          <div class="header-icon">
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <rect x="2" y="4" width="20" height="16" rx="2" ry="2"></rect>
              <path d="M6 8h.001"></path>
              <path d="M10 8h.001"></path>
              <path d="M14 8h.001"></path>
              <path d="M18 8h.001"></path>
              <path d="M8 12h.001"></path>
              <path d="M12 12h.001"></path>
              <path d="M16 12h.001"></path>
              <path d="M7 16h10"></path>
            </svg>
          </div>
          <h2 id="dialog-title">Keyboard Shortcuts</h2>
          <button class="close-btn" onclick={onClose} aria-label="Close">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>

        <div class="content">
          <div class="groups-grid">
            {#each shortcutGroups as group, groupIndex (group.title)}
              <div class="group" style="--group-index: {groupIndex}">
                <h3 class="group-title">
                  <span class="group-icon">{group.icon}</span>
                  {group.title}
                </h3>
                <div class="shortcuts">
                  {#each group.shortcuts as shortcut, shortcutIndex (shortcut.description)}
                    <div class="shortcut-row" style="--shortcut-index: {shortcutIndex}">
                      <span class="description">{shortcut.description}</span>
                      <span class="keys">
                        {#each shortcut.keys as key, i (i)}
                          <kbd>{key}</kbd>
                          {#if i < shortcut.keys.length - 1}
                            <span class="separator"></span>
                          {/if}
                        {/each}
                      </span>
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        </div>

        <div class="footer">
          <div class="brand">
            <span class="brand-text">kiri</span>
            <span class="brand-kanji">霧</span>
          </div>
          <span class="hint">
            <kbd>Esc</kbd>
            <span>to close</span>
          </span>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .backdrop.visible {
    opacity: 1;
  }

  .dialog {
    position: relative;
    max-width: 640px;
    width: 90%;
    max-height: 85vh;
    transform: translateY(20px) scale(0.95);
    opacity: 0;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .dialog.visible {
    transform: translateY(0) scale(1);
    opacity: 1;
  }

  .dialog-glow {
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: calc(var(--radius-xl) + 2px);
    opacity: 0.06;
    filter: blur(6px);
    z-index: -1;
    transition: opacity 0.3s ease;
  }

  .dialog:hover .dialog-glow {
    opacity: 0.1;
  }

  .dialog-content {
    background: var(--bg-glass);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-xl);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-lg);
  }

  .header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.2);
  }

  .header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-color);
  }

  .header h2 {
    flex: 1;
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--font-sans);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .close-btn:hover {
    background: var(--bg-tertiary);
    color: var(--accent-color);
  }

  .close-btn svg {
    transition: transform var(--transition-fast);
  }

  .close-btn:hover svg {
    transform: scale(1.1);
  }

  .close-btn:active {
    transform: scale(0.95);
    transition: transform 100ms ease;
  }

  .content {
    padding: var(--space-5);
    overflow-y: auto;
  }

  .groups-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-5);
  }

  @media (max-width: 560px) {
    .groups-grid {
      grid-template-columns: 1fr;
    }
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    animation: groupFadeIn 0.4s ease backwards;
    animation-delay: calc(var(--group-index) * 30ms);
  }

  @keyframes groupFadeIn {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .group-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    transition: color var(--transition-fast);
  }

  .group-icon {
    font-size: 14px;
    display: inline-block;
    transition: transform var(--transition-fast);
  }

  .group:hover .group-icon {
    transform: scale(1.1);
  }

  .group:hover .group-title {
    color: var(--text-secondary);
  }

  .shortcuts {
    display: flex;
    flex-direction: column;
    background: rgba(0, 0, 0, 0.15);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-3);
    border-bottom: 1px solid var(--border-subtle);
    transition: background var(--transition-fast);
    animation: shortcutSlide 0.3s ease backwards;
    animation-delay: calc(var(--group-index) * 30ms + var(--shortcut-index) * 30ms);
  }

  @keyframes shortcutSlide {
    from {
      opacity: 0;
      transform: translateX(-8px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .shortcut-row:last-child {
    border-bottom: none;
  }

  .shortcut-row:hover {
    background: rgba(125, 211, 252, 0.05);
  }

  .shortcut-row:hover::before {
    content: '';
    position: absolute;
    left: 0;
    top: 25%;
    bottom: 25%;
    width: 2px;
    background: var(--accent-color);
    border-radius: 1px;
  }

  .shortcut-row {
    position: relative;
  }

  .description {
    font-size: 13px;
    color: var(--text-primary);
  }

  .keys {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 26px;
    height: 26px;
    padding: 0 8px;
    background: linear-gradient(180deg, var(--bg-tertiary) 0%, var(--bg-secondary) 100%);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-size: 11px;
    font-family: var(--font-mono);
    font-weight: 500;
    color: var(--text-primary);
    box-shadow:
      0 2px 0 var(--bg-primary),
      inset 0 1px 0 rgba(255, 255, 255, 0.05);
    transition: all var(--transition-fast);
  }

  .shortcut-row:hover kbd {
    transform: translateY(-1px);
    box-shadow:
      0 3px 0 var(--bg-primary),
      inset 0 1px 0 rgba(255, 255, 255, 0.05);
    border-color: var(--accent-subtle);
    color: var(--accent-color);
  }

  .separator {
    width: 4px;
    height: 1px;
    background: var(--text-muted);
    opacity: 0.5;
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--border-subtle);
    background: rgba(0, 0, 0, 0.2);
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 4px;
  }

  .brand-text {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
  }

  .brand-kanji {
    font-size: 10px;
    color: var(--text-muted);
    opacity: 0.5;
  }

  .hint {
    font-size: 11px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .hint kbd {
    min-width: auto;
    height: 20px;
    padding: 0 6px;
    font-size: 10px;
    box-shadow: none;
  }
</style>
