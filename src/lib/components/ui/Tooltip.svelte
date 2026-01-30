<script lang="ts">
  interface Props {
    text: string;
    position?: 'top' | 'bottom' | 'left' | 'right';
    delay?: number;
    children: import('svelte').Snippet;
  }

  let { text, position = 'top', delay = 300, children }: Props = $props();

  let visible = $state(false);
  let triggerRef = $state<HTMLDivElement | null>(null);
  let tooltipRef = $state<HTMLDivElement | null>(null);
  let timeoutId: ReturnType<typeof setTimeout>;

  function show() {
    timeoutId = setTimeout(() => {
      visible = true;
    }, delay);
  }

  function hide() {
    clearTimeout(timeoutId);
    visible = false;
  }

  function handleMouseEnter() {
    show();
  }

  function handleMouseLeave() {
    hide();
  }

  function handleFocus() {
    show();
  }

  function handleBlur() {
    hide();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="tooltip-wrapper"
  bind:this={triggerRef}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  onfocusin={handleFocus}
  onfocusout={handleBlur}
>
  {@render children()}

  {#if visible}
    <div
      class="tooltip"
      class:top={position === 'top'}
      class:bottom={position === 'bottom'}
      class:left={position === 'left'}
      class:right={position === 'right'}
      bind:this={tooltipRef}
      role="tooltip"
    >
      <div class="tooltip-glow"></div>
      <div class="tooltip-content">
        {text}
      </div>
      <div class="tooltip-arrow"></div>
    </div>
  {/if}
</div>

<style>
  .tooltip-wrapper {
    position: relative;
    display: inline-flex;
  }

  .tooltip {
    position: absolute;
    z-index: 9999;
    pointer-events: none;
    animation: tooltipFadeIn 0.2s ease;
  }

  @keyframes tooltipFadeIn {
    from {
      opacity: 0;
      transform: scale(0.92);
      filter: blur(2px);
    }
    to {
      opacity: 1;
      transform: scale(1);
      filter: blur(0);
    }
  }

  .tooltip-glow {
    position: absolute;
    inset: -1px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: calc(var(--radius-md) + 1px);
    opacity: 0.06;
    filter: blur(2px);
    z-index: -1;
    transition: opacity var(--transition-fast);
  }

  .tooltip:hover .tooltip-glow {
    opacity: 0.1;
  }

  .tooltip-content {
    padding: var(--space-2) var(--space-3);
    background: var(--bg-glass);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-md);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    box-shadow: var(--shadow-md);
  }

  .tooltip-arrow {
    position: absolute;
    width: 8px;
    height: 8px;
    background: var(--bg-glass);
    border: 1px solid var(--border-glow);
    transform: rotate(45deg);
  }

  /* Position: top */
  .tooltip.top {
    bottom: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
    transform-origin: center bottom;
  }

  .tooltip.top .tooltip-arrow {
    bottom: -5px;
    left: 50%;
    transform: translateX(-50%) rotate(45deg);
    border-top: none;
    border-left: none;
  }

  /* Position: bottom */
  .tooltip.bottom {
    top: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
    transform-origin: center top;
  }

  .tooltip.bottom .tooltip-arrow {
    top: -5px;
    left: 50%;
    transform: translateX(-50%) rotate(45deg);
    border-bottom: none;
    border-right: none;
  }

  /* Position: left */
  .tooltip.left {
    right: calc(100% + 8px);
    top: 50%;
    transform: translateY(-50%);
    transform-origin: right center;
  }

  .tooltip.left .tooltip-arrow {
    right: -5px;
    top: 50%;
    transform: translateY(-50%) rotate(45deg);
    border-top: none;
    border-right: none;
  }

  /* Position: right */
  .tooltip.right {
    left: calc(100% + 8px);
    top: 50%;
    transform: translateY(-50%);
    transform-origin: left center;
  }

  .tooltip.right .tooltip-arrow {
    left: -5px;
    top: 50%;
    transform: translateY(-50%) rotate(45deg);
    border-bottom: none;
    border-left: none;
  }
</style>
