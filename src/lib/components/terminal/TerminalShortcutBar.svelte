<script lang="ts">
  import type { TerminalShortcut } from '@/lib/stores/shortcutStore.svelte';

  interface Props {
    visible: boolean;
    shortcuts: TerminalShortcut[];
    onSend: (text: string, withEnter: boolean) => void;
    onSettingsClick: () => void;
  }

  let { visible, shortcuts, onSend, onSettingsClick }: Props = $props();

  const replies = $derived(shortcuts.filter((s) => s.type === 'reply'));
  const commands = $derived(shortcuts.filter((s) => s.type === 'command'));

  function handleClick(event: MouseEvent, shortcut: TerminalShortcut) {
    const withEnter = !event.shiftKey;
    onSend(shortcut.text, withEnter);
  }
</script>

{#if visible}
  <div class="shortcut-bar">
    <!-- Row 1: Quick Reply -->
    <div class="shortcut-row">
      <span class="bar-label">REPLY</span>
      <div class="shortcut-buttons">
        {#each replies as shortcut (shortcut.id)}
          <button
            class="shortcut-btn reply-btn"
            onclick={(e) => handleClick(e, shortcut)}
            title="{shortcut.label} (Shift+click: input only)"
          >
            {shortcut.label}
          </button>
        {/each}
      </div>
    </div>

    <!-- Row 2: Commands (only if any exist) -->
    {#if commands.length > 0}
      <div class="shortcut-row">
        <span class="bar-label">CMD</span>
        <div class="shortcut-buttons">
          {#each commands as shortcut (shortcut.id)}
            <button
              class="shortcut-btn command-btn"
              onclick={(e) => handleClick(e, shortcut)}
              title="{shortcut.label} (Shift+click: input only)"
            >
              {shortcut.label}
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Settings button -->
    <button
      class="settings-btn"
      onclick={onSettingsClick}
      title="Shortcut Settings"
      aria-label="Shortcut Settings"
    >
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <circle cx="12" cy="12" r="3" />
        <path
          d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
        />
      </svg>
    </button>
  </div>
{/if}

<style>
  .shortcut-bar {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px var(--space-3);
    background: linear-gradient(180deg, rgba(125, 211, 252, 0.08) 0%, rgba(13, 17, 23, 0.85) 100%);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border-top: 1px solid rgba(125, 211, 252, 0.25);
    animation: slideUp 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    position: relative;
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .bar-label {
    flex-shrink: 0;
    width: 38px;
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    user-select: none;
    text-align: right;
  }

  .shortcut-buttons {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex: 1;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .shortcut-buttons::-webkit-scrollbar {
    display: none;
  }

  .shortcut-btn {
    flex-shrink: 0;
    padding: 4px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
    border-radius: 12px;
    cursor: pointer;
    white-space: nowrap;
    transition:
      background var(--transition-fast),
      color var(--transition-fast),
      border-color var(--transition-fast),
      box-shadow var(--transition-fast),
      transform var(--transition-fast);
  }

  /* Reply buttons — accent blue */
  .reply-btn {
    color: var(--accent-color);
    background: rgba(125, 211, 252, 0.1);
    border: 1px solid rgba(125, 211, 252, 0.3);
    text-shadow: 0 0 12px rgba(125, 211, 252, 0.3);
    box-shadow: 0 0 8px rgba(125, 211, 252, 0.08);
  }

  .reply-btn:hover {
    color: #fff;
    background: rgba(125, 211, 252, 0.22);
    border-color: rgba(125, 211, 252, 0.5);
    box-shadow:
      0 0 16px rgba(125, 211, 252, 0.25),
      0 0 4px rgba(125, 211, 252, 0.15);
  }

  /* Command buttons — secondary purple */
  .command-btn {
    color: var(--accent2-color, #c4b5fd);
    background: rgba(196, 181, 253, 0.08);
    border: 1px solid rgba(196, 181, 253, 0.25);
    text-shadow: 0 0 12px rgba(196, 181, 253, 0.25);
    box-shadow: 0 0 8px rgba(196, 181, 253, 0.06);
  }

  .command-btn:hover {
    color: #fff;
    background: rgba(196, 181, 253, 0.2);
    border-color: rgba(196, 181, 253, 0.45);
    box-shadow:
      0 0 16px rgba(196, 181, 253, 0.2),
      0 0 4px rgba(196, 181, 253, 0.12);
  }

  .shortcut-btn:active {
    transform: scale(0.96);
  }

  .settings-btn {
    position: absolute;
    top: 6px;
    right: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    color: var(--text-muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition:
      color var(--transition-fast),
      background var(--transition-fast),
      border-color var(--transition-fast);
  }

  .settings-btn:hover {
    color: var(--text-secondary);
    background: rgba(125, 211, 252, 0.08);
    border-color: var(--border-color);
  }

  .settings-btn:active {
    transform: scale(0.92);
  }
</style>
