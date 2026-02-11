<script lang="ts">
  import type { TerminalPane } from '@/lib/stores/tabStore';
  import { tabStore } from '@/lib/stores/tabStore';
  import Terminal from './Terminal.svelte';
  import TerminalContainer from './TerminalContainer.svelte';

  interface Props {
    tabId: string;
    pane: TerminalPane;
    cwd?: string | null;
    isOnlyPane?: boolean;
  }

  let { tabId, pane, cwd = null, isOnlyPane = false }: Props = $props();

  let containerRef = $state<HTMLDivElement | null>(null);
  let isDragging = $state(false);
  let dragIndex = $state(-1);
  let resizeThrottleTimeout: ReturnType<typeof setTimeout> | null = null;

  function handleSplitHorizontal(paneId: string) {
    tabStore.splitPane(tabId, paneId, 'horizontal');
    // Trigger resize after split
    requestAnimationFrame(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    });
  }

  function handleSplitVertical(paneId: string) {
    tabStore.splitPane(tabId, paneId, 'vertical');
    // Trigger resize after split
    requestAnimationFrame(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    });
  }

  function handleClose(closingPaneId: string) {
    tabStore.closePane(tabId, closingPaneId);
    // Trigger resize after close
    requestAnimationFrame(() => {
      window.dispatchEvent(new Event('terminal-resize'));
    });
  }

  function handleDividerMouseDown(event: MouseEvent, index: number) {
    if (pane.type !== 'split') return;

    event.preventDefault();
    isDragging = true;
    dragIndex = index;

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging || !containerRef || pane.type !== 'split') return;

      const rect = containerRef.getBoundingClientRect();
      const isVertical = pane.direction === 'vertical';

      // Calculate position as percentage
      const position = isVertical
        ? ((e.clientX - rect.left) / rect.width) * 100
        : ((e.clientY - rect.top) / rect.height) * 100;

      // For 2 panes, distribute sizes based on divider position
      const totalChildren = pane.children.length;
      const newSizes: number[] = [];

      // Calculate size for each pane based on divider positions
      // For simplicity, handle the common 2-pane case
      if (totalChildren === 2 && dragIndex === 0) {
        const size1 = Math.max(10, Math.min(90, position));
        const size2 = 100 - size1;
        newSizes.push(size1, size2);
      } else {
        // For more complex cases, just update the sizes proportionally
        const beforeSize = position;
        const afterSize = 100 - position;

        // Distribute to panes before and after the divider
        let beforeTotal = 0;
        let afterTotal = 0;

        for (let i = 0; i <= dragIndex; i++) {
          beforeTotal += pane.sizes[i];
        }
        for (let i = dragIndex + 1; i < totalChildren; i++) {
          afterTotal += pane.sizes[i];
        }

        for (let i = 0; i < totalChildren; i++) {
          if (i <= dragIndex) {
            newSizes.push((pane.sizes[i] / beforeTotal) * beforeSize);
          } else {
            newSizes.push((pane.sizes[i] / afterTotal) * afterSize);
          }
        }
      }

      tabStore.updatePaneSizes(tabId, pane.id, newSizes);

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
    <Terminal
      {tabId}
      paneId={pane.id}
      cwd={pane.cwd || cwd}
      showControls={true}
      onSplitHorizontal={() => handleSplitHorizontal(pane.id)}
      onSplitVertical={() => handleSplitVertical(pane.id)}
      onClose={isOnlyPane ? undefined : () => handleClose(pane.id)}
    />
  {:else}
    <div
      bind:this={containerRef}
      class="split-container"
      class:horizontal={pane.direction === 'horizontal'}
      class:vertical={pane.direction === 'vertical'}
      class:dragging={isDragging}
    >
      {#each pane.children as child, index (child.type === 'terminal' ? child.id : index)}
        <div class="split-pane" style="flex: 0 0 {pane.sizes[index]}%;">
          <TerminalContainer {tabId} pane={child} {cwd} isOnlyPane={false} />
        </div>
        {#if index < pane.children.length - 1}
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
