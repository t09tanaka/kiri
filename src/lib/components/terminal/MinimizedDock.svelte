<script lang="ts">
  import { onDestroy } from 'svelte';
  import { terminalStore, type TerminalPaneLeaf } from '@/lib/stores/terminalStore';
  import { floatingPaneStore } from '@/lib/stores/floatingPaneStore';

  // Panes parked in the dock. Kept in a local `$state` because the minimized
  // set is tracked outside the pane tree and notifies via the store's
  // subscribe channel rather than by replacing the tree object.
  let leaves = $state<TerminalPaneLeaf[]>([]);
  const unsubscribe = terminalStore.subscribe(() => {
    leaves = terminalStore.minimizedLeaves();
  });
  onDestroy(unsubscribe);

  let floatingId = $state<string | null>(null);
  const unsubscribeFloating = floatingPaneStore.subscribe((id) => {
    floatingId = id;
  });
  onDestroy(unsubscribeFloating);

  function labelFor(leaf: TerminalPaneLeaf): string {
    if (leaf.name) return leaf.name;
    if (leaf.terminalId !== null) return `Terminal ${leaf.terminalId}`;
    return 'Terminal';
  }

  function handleChipClick(paneId: string) {
    floatingPaneStore.toggle(paneId);
  }

  function handleChipClose(event: MouseEvent, paneId: string) {
    event.stopPropagation();
    if (floatingId === paneId) floatingPaneStore.close();
    terminalStore.closePane(paneId);
    // The layout reclaims the dock's vacancy is irrelevant, but other panes
    // may shift if this was the last dock entry; nudge a refit to be safe.
    requestAnimationFrame(() => window.dispatchEvent(new Event('terminal-resize')));
  }
</script>

{#if leaves.length > 0}
  <div class="dock" role="toolbar" aria-label="Minimized panes">
    <span class="dock-icon" aria-hidden="true">
      <svg
        width="13"
        height="13"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <rect x="3" y="14" width="18" height="6" rx="1.5" />
        <path d="M12 3v7" />
        <path d="M8.5 7.5 12 11l3.5-3.5" />
      </svg>
    </span>
    <div class="dock-chips">
      {#each leaves as leaf (leaf.id)}
        <div class="chip" class:active={floatingId === leaf.id} data-dock-chip>
          <button
            class="chip-main"
            onclick={() => handleChipClick(leaf.id)}
            title={`Peek ${labelFor(leaf)}`}
            aria-label={`Peek ${labelFor(leaf)}`}
            aria-pressed={floatingId === leaf.id}
          >
            <span
              class="chip-dot"
              style:--pane-color={leaf.color
                ? `var(--pane-color-${leaf.color})`
                : 'var(--text-muted)'}
              aria-hidden="true"
            ></span>
            <span class="chip-name">{labelFor(leaf)}</span>
          </button>
          <button
            class="chip-close"
            onclick={(e) => handleChipClose(e, leaf.id)}
            title={`Close ${labelFor(leaf)}`}
            aria-label={`Close ${labelFor(leaf)}`}
          >
            <svg
              width="11"
              height="11"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.4"
              stroke-linecap="round"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .dock {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 12px;
    background: rgba(10, 12, 16, 0.72);
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    border-top: 1px solid rgba(125, 211, 252, 0.12);
    flex-shrink: 0;
    overflow: hidden;
  }

  /* Faint shine line tracing the dock's top edge — echoes the modal
     shine-line motif from the mist design language. */
  .dock::before {
    content: '';
    position: absolute;
    top: 0;
    left: 8%;
    right: 8%;
    height: 1px;
    background: linear-gradient(90deg, transparent, rgba(125, 211, 252, 0.4), transparent);
    opacity: 0.5;
    pointer-events: none;
  }

  .dock-icon {
    display: flex;
    align-items: center;
    color: var(--text-muted);
    opacity: 0.6;
    flex-shrink: 0;
  }

  .dock-chips {
    display: flex;
    align-items: center;
    gap: 8px;
    overflow-x: auto;
    scrollbar-width: thin;
    scrollbar-color: rgba(125, 211, 252, 0.15) transparent;
    padding-bottom: 1px;
  }

  .dock-chips::-webkit-scrollbar {
    height: 5px;
  }

  .dock-chips::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.14);
    border-radius: 4px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    background: rgba(125, 211, 252, 0.05);
    border: 1px solid rgba(125, 211, 252, 0.12);
    border-radius: var(--radius-md, 8px);
    overflow: hidden;
    transition:
      border-color var(--transition-fast),
      background var(--transition-fast),
      box-shadow var(--transition-fast),
      transform var(--transition-fast);
  }

  .chip:hover {
    background: rgba(125, 211, 252, 0.09);
    border-color: rgba(125, 211, 252, 0.28);
    transform: translateY(-1px);
  }

  /* The pane currently floating reads as "lit" — accent border plus a soft
     outer glow, matching the focus treatment used elsewhere. */
  .chip.active {
    background: rgba(125, 211, 252, 0.12);
    border-color: var(--accent-color);
    box-shadow:
      0 0 0 1px rgba(125, 211, 252, 0.25),
      0 2px 12px -2px rgba(125, 211, 252, 0.35);
  }

  .chip-main {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 4px 5px 10px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-family: 'IBM Plex Mono', 'JetBrains Mono', monospace;
    font-size: 11px;
    letter-spacing: 0.03em;
    cursor: pointer;
    max-width: 180px;
  }

  .chip.active .chip-main {
    color: var(--text-primary);
  }

  .chip-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--pane-color);
    box-shadow: 0 0 6px 0.5px color-mix(in srgb, var(--pane-color) 55%, transparent);
    flex-shrink: 0;
  }

  .chip-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 26px;
    padding: 0;
    margin-right: 2px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm, 4px);
    color: var(--text-muted);
    cursor: pointer;
    opacity: 0.55;
    transition:
      opacity var(--transition-fast),
      color var(--transition-fast),
      background var(--transition-fast);
  }

  .chip-close:hover {
    opacity: 1;
    color: #f87171;
    background: rgba(248, 113, 113, 0.12);
  }
</style>
