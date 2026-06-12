<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { TerminalPane } from '@/lib/stores/terminalStore';
  import { terminalStore } from '@/lib/stores/terminalStore';
  import Terminal from './Terminal.svelte';
  import TerminalContainer from './TerminalContainer.svelte';

  interface Props {
    pane: TerminalPane;
    cwd?: string | null;
    isOnlyPane?: boolean;
  }

  let { pane, cwd = null, isOnlyPane = false }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let isDragging = $state(false);
  let dragIndex = $state(-1);
  let resizeThrottleTimeout: ReturnType<typeof setTimeout> | null = null;

  // The minimized set lives outside the pane tree (terminalStore notifies
  // without changing the tree's identity), so subscribe explicitly and bump
  // a counter the derived layout depends on to stay reactive to it.
  let minimizedVersion = $state(0);
  const unsubscribeMinimized = terminalStore.subscribe(() => {
    minimizedVersion++;
  });
  onDestroy(unsubscribeMinimized);

  /** A subtree is visible when at least one of its leaves is not minimized. */
  function isPaneVisible(target: TerminalPane): boolean {
    if (target.type === 'terminal') return !terminalStore.isMinimized(target.id);
    return target.children.some(isPaneVisible);
  }

  // This leaf should not render inside the layout while it is minimized —
  // the footer dock (and the floating peek) own it instead.
  const selfMinimized = $derived.by(() => {
    void minimizedVersion;
    return pane.type === 'terminal' && terminalStore.isMinimized(pane.id);
  });

  // Offer the minimize affordance only while more than one pane is visible;
  // minimizing the last one would leave an empty layout.
  const canMinimize = $derived.by(() => {
    void minimizedVersion;
    return terminalStore.visiblePaneCount() > 1;
  });

  // Visible children of a split with sizes renormalized over just the
  // visible subset, keeping the original index so divider drags can write
  // back into the full `pane.sizes` array.
  const visibleChildren = $derived.by(() => {
    void minimizedVersion;
    if (pane.type !== 'split') return [];
    const items: { child: TerminalPane; size: number; originalIndex: number }[] = [];
    let total = 0;
    pane.children.forEach((child, originalIndex) => {
      if (isPaneVisible(child)) {
        items.push({ child, size: pane.sizes[originalIndex], originalIndex });
        total += pane.sizes[originalIndex];
      }
    });
    if (total > 0) {
      for (const item of items) item.size = (item.size / total) * 100;
    }
    return items;
  });

  function handleSplitHorizontal(paneId: string) {
    terminalStore.splitPane(paneId, 'horizontal');
    // Trigger resize after split
    requestAnimationFrame(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    });
  }

  function handleSplitVertical(paneId: string) {
    terminalStore.splitPane(paneId, 'vertical');
    // Trigger resize after split
    requestAnimationFrame(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    });
  }

  function handleClose(closingPaneId: string) {
    terminalStore.closePane(closingPaneId);
    // Trigger resize after close
    requestAnimationFrame(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    });
  }

  function handleMinimize(minimizingPaneId: string) {
    terminalStore.setMinimized(minimizingPaneId, true);
    // The remaining visible panes grow to fill the freed space; refit them.
    requestAnimationFrame(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    });
  }

  function handleDividerMouseDown(event: MouseEvent, index: number) {
    if (pane.type !== 'split') return;

    event.preventDefault();
    isDragging = true;
    dragIndex = index;

    // Resize operates over the currently visible subset; minimized panes are
    // not rendered, so the divider at visible `index` sits between visible
    // items only. We map results back into the full `pane.sizes` array.
    const visible = visibleChildren;

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging || !containerRef || pane.type !== 'split') return;

      const rect = containerRef.getBoundingClientRect();
      const isVertical = pane.direction === 'vertical';

      // Calculate position as percentage
      const position = isVertical
        ? ((e.clientX - rect.left) / rect.width) * 100
        : ((e.clientY - rect.top) / rect.height) * 100;

      // Distribute sizes among the visible panes based on divider position.
      const visibleCount = visible.length;
      const visibleSizes = visible.map((item) => item.size);
      const newVisibleSizes: number[] = [];

      // Common 2-visible-pane case: split directly at the divider position.
      if (visibleCount === 2 && dragIndex === 0) {
        const size1 = Math.max(10, Math.min(90, position));
        newVisibleSizes.push(size1, 100 - size1);
      } else {
        const beforeSize = position;
        const afterSize = 100 - position;

        let beforeTotal = 0;
        let afterTotal = 0;
        for (let i = 0; i <= dragIndex; i++) beforeTotal += visibleSizes[i];
        for (let i = dragIndex + 1; i < visibleCount; i++) afterTotal += visibleSizes[i];

        for (let i = 0; i < visibleCount; i++) {
          if (i <= dragIndex) {
            newVisibleSizes.push((visibleSizes[i] / beforeTotal) * beforeSize);
          } else {
            newVisibleSizes.push((visibleSizes[i] / afterTotal) * afterSize);
          }
        }
      }

      // Scatter the new visible sizes back into the full sizes array; hidden
      // (minimized) panes keep their stored size and are renormalized away on
      // render anyway.
      const fullSizes = [...pane.sizes];
      visible.forEach((item, i) => {
        fullSizes[item.originalIndex] = newVisibleSizes[i];
      });

      terminalStore.updatePaneSizes(pane.id, fullSizes);

      // Throttled resize event dispatch during drag
      if (!resizeThrottleTimeout) {
        resizeThrottleTimeout = setTimeout(() => {
          window.dispatchEvent(new Event('terminal-resize'));
          resizeThrottleTimeout = null;
        }, 16);
      }
    };

    const handleMouseUp = () => {
      isDragging = false;
      dragIndex = -1;
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      // Clear throttle timeout and trigger final resize
      if (resizeThrottleTimeout) {
        clearTimeout(resizeThrottleTimeout);
        resizeThrottleTimeout = null;
      }
      window.dispatchEvent(new Event('terminal-resize'));
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    document.body.style.cursor = pane.direction === 'vertical' ? 'col-resize' : 'row-resize';
    document.body.style.userSelect = 'none';
  }
</script>

{#key pane.type}
  {#if pane.type === 'terminal'}
    {#if !selfMinimized}
      <Terminal
        paneId={pane.id}
        cwd={pane.cwd || cwd}
        name={pane.name}
        color={pane.color}
        showControls={true}
        onSplitHorizontal={() => handleSplitHorizontal(pane.id)}
        onSplitVertical={() => handleSplitVertical(pane.id)}
        onMinimize={canMinimize ? () => handleMinimize(pane.id) : undefined}
        onClose={isOnlyPane ? undefined : () => handleClose(pane.id)}
      />
    {/if}
  {:else}
    <div
      bind:this={containerRef}
      class="split-container"
      class:horizontal={pane.direction === 'horizontal'}
      class:vertical={pane.direction === 'vertical'}
      class:dragging={isDragging}
    >
      {#each visibleChildren as item, index (item.child.type === 'terminal' ? item.child.id : item.originalIndex)}
        <div class="split-pane" style="flex: 0 0 {item.size}%;">
          <TerminalContainer pane={item.child} {cwd} isOnlyPane={false} />
        </div>
        {#if index < visibleChildren.length - 1}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            class="split-divider"
            class:horizontal={pane.direction === 'horizontal'}
            class:vertical={pane.direction === 'vertical'}
            onmousedown={(e) => handleDividerMouseDown(e, index)}
            role="separator"
            aria-orientation={pane.direction}
          ></div>
        {/if}
      {/each}
    </div>
  {/if}
{/key}

<style>
  .split-container {
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .split-container.horizontal {
    flex-direction: column;
  }

  .split-container.vertical {
    flex-direction: row;
  }

  .split-container.dragging {
    cursor: inherit;
  }

  .split-pane {
    overflow: hidden;
    min-width: 100px;
    min-height: 100px;
  }

  .split-divider {
    flex-shrink: 0;
    background: rgba(125, 211, 252, 0.15);
    transition: background var(--transition-fast);
    z-index: 10;
  }

  .split-divider:hover,
  .split-divider:active {
    background: rgba(125, 211, 252, 0.4);
  }

  .split-divider.horizontal {
    height: 4px;
    width: 100%;
    cursor: row-resize;
  }

  .split-divider.vertical {
    width: 4px;
    height: 100%;
    cursor: col-resize;
  }
</style>
