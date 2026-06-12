<script lang="ts">
  import { onDestroy } from 'svelte';
  import Terminal from './Terminal.svelte';
  import { floatingPaneStore } from '@/lib/stores/floatingPaneStore';
  import { terminalStore, type TerminalPaneLeaf } from '@/lib/stores/terminalStore';

  let floatingId = $state<string | null>(null);
  const unsubscribeFloating = floatingPaneStore.subscribe((id) => {
    floatingId = id;
  });
  onDestroy(unsubscribeFloating);

  // The leaf's label/color can change (rename, color pick) while it floats,
  // so recompute when either the floating target or the tree changes.
  let treeVersion = $state(0);
  const unsubscribeTree = terminalStore.subscribe(() => {
    treeVersion++;
  });
  onDestroy(unsubscribeTree);

  const leaf = $derived.by<TerminalPaneLeaf | null>(() => {
    void treeVersion;
    return floatingId ? terminalStore.getLeaf(floatingId) : null;
  });

  let windowEl = $state<HTMLDivElement | null>(null);

  function labelFor(target: TerminalPaneLeaf): string {
    if (target.name) return target.name;
    if (target.terminalId !== null) return `Terminal ${target.terminalId}`;
    return 'Terminal';
  }

  function close() {
    floatingPaneStore.close();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    }
  }

  // Peek semantics: a pointer press anywhere outside the floating window
  // dismisses it. Dock chips are exempt — they manage their own toggle, so a
  // press there should switch/close via the dock rather than racing with us.
  $effect(() => {
    if (!floatingId) return;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as HTMLElement | null;
      if (!target || !windowEl) return;
      if (windowEl.contains(target)) return;
      if (target.closest('[data-dock-chip]')) return;
      close();
    };
    document.addEventListener('pointerdown', onPointerDown, true);
    return () => document.removeEventListener('pointerdown', onPointerDown, true);
  });

  // Give the shell focus the moment it mounts so keyboard (Esc) works even
  // before xterm claims the textarea, and so the window reads as active.
  $effect(() => {
    if (windowEl) {
      const el = windowEl;
      requestAnimationFrame(() => el.focus());
    }
  });
</script>

{#if leaf}
  <div class="float-layer">
    <div
      class="float-window"
      bind:this={windowEl}
      tabindex="-1"
      role="dialog"
      aria-label={`${labelFor(leaf)} — floating terminal`}
      onkeydown={handleKeydown}
    >
      <div class="modal-glow" aria-hidden="true"></div>
      <div class="float-header">
        <span class="float-title">
          <span
            class="float-dot"
            style:--pane-color={leaf.color
              ? `var(--pane-color-${leaf.color})`
              : 'var(--text-muted)'}
            aria-hidden="true"
          ></span>
          <span class="float-name">{labelFor(leaf)}</span>
          <span class="float-tag">floating</span>
        </span>
        <button class="float-close" onclick={close} title="Return to dock (Esc)" aria-label="Close">
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          >
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>
      <div class="float-body">
        {#key leaf.id}
          <Terminal
            paneId={leaf.id}
            cwd={leaf.cwd}
            name={leaf.name}
            color={leaf.color}
            showControls={false}
          />
        {/key}
      </div>
    </div>
  </div>
{/if}

<style>
  /* Non-blocking overlay: the layer itself ignores pointer events so the
     layout behind stays clickable (and a click there dismisses the peek);
     only the window opts back in. */
  .float-layer {
    position: absolute;
    inset: 0;
    z-index: 50;
    pointer-events: none;
  }

  .float-window {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(880px, 66%);
    height: min(540px, 64%);
    display: flex;
    flex-direction: column;
    background: rgba(12, 14, 20, 0.86);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid rgba(125, 211, 252, 0.2);
    border-radius: var(--radius-xl, 14px);
    box-shadow:
      0 24px 64px -16px rgba(0, 0, 0, 0.7),
      0 0 0 1px rgba(125, 211, 252, 0.06);
    overflow: hidden;
    pointer-events: auto;
    outline: none;
    animation: floatIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes floatIn {
    from {
      opacity: 0;
      transform: translate(-50%, -50%) translateY(12px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translate(-50%, -50%) translateY(0) scale(1);
    }
  }

  /* Soft gradient halo bleeding past the frame — the mist "glow" motif. */
  .modal-glow {
    position: absolute;
    inset: -2px;
    border-radius: calc(var(--radius-xl, 14px) + 2px);
    background: linear-gradient(135deg, var(--accent-color), var(--accent2-color));
    opacity: 0.07;
    filter: blur(6px);
    pointer-events: none;
    z-index: 0;
  }

  /* Shine line along the top edge. */
  .float-window::before {
    content: '';
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent-color), transparent);
    opacity: 0.6;
    z-index: 2;
    pointer-events: none;
  }

  .float-header {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    height: var(--tabbar-height, 44px);
    padding: 0 8px 0 14px;
    background: rgba(10, 12, 16, 0.6);
    border-bottom: 1px solid rgba(125, 211, 252, 0.1);
    flex-shrink: 0;
    cursor: default;
  }

  .float-title {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-family: 'IBM Plex Mono', 'JetBrains Mono', monospace;
    font-size: 12px;
    letter-spacing: 0.03em;
    color: var(--text-primary);
  }

  .float-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--pane-color);
    box-shadow: 0 0 6px 0.5px color-mix(in srgb, var(--pane-color) 60%, transparent);
    flex-shrink: 0;
  }

  .float-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .float-tag {
    flex-shrink: 0;
    padding: 1px 7px;
    font-size: 9px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--accent-color);
    background: rgba(125, 211, 252, 0.1);
    border: 1px solid rgba(125, 211, 252, 0.18);
    border-radius: 999px;
  }

  .float-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm, 4px);
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition:
      color var(--transition-fast),
      background var(--transition-fast);
  }

  .float-close:hover {
    color: #f87171;
    background: rgba(248, 113, 113, 0.12);
  }

  .float-body {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
</style>
