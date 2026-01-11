<script lang="ts">
  interface Props {
    value?: number;
    indeterminate?: boolean;
    showLabel?: boolean;
    size?: 'sm' | 'md';
  }

  let { value = 0, indeterminate = false, showLabel = false, size = 'md' }: Props = $props();

  const clampedValue = $derived(Math.min(100, Math.max(0, value)));
</script>

<div class="progress-container" class:sm={size === 'sm'} class:md={size === 'md'}>
  <div class="progress-track">
    <div class="track-glow"></div>
    {#if indeterminate}
      <div class="progress-indeterminate">
        <div class="bar bar-1"></div>
        <div class="bar bar-2"></div>
      </div>
    {:else}
      <div class="progress-fill" style="width: {clampedValue}%">
        <div class="fill-glow"></div>
        <div class="fill-shimmer"></div>
      </div>
    {/if}
  </div>
  {#if showLabel && !indeterminate}
    <span class="progress-label">{Math.round(clampedValue)}%</span>
  {/if}
</div>

<style>
  .progress-container {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
  }

  .progress-track {
    position: relative;
    flex: 1;
    height: 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    overflow: hidden;
  }

  .sm .progress-track {
    height: 4px;
    border-radius: 2px;
  }

  .track-glow {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(125, 211, 252, 0.03) 50%,
      transparent 100%
    );
  }

  .progress-fill {
    position: relative;
    height: 100%;
    background: linear-gradient(
      90deg,
      var(--accent-color) 0%,
      var(--accent2-color) 50%,
      var(--accent-color) 100%
    );
    background-size: 200% 100%;
    border-radius: inherit;
    transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    animation: progressGradient 3s ease-in-out infinite;
  }

  @keyframes progressGradient {
    0%,
    100% {
      background-position: 0% 50%;
    }
    50% {
      background-position: 100% 50%;
    }
  }

  .fill-glow {
    position: absolute;
    inset: 0;
    background: rgba(125, 211, 252, 0.06);
    opacity: 0.3;
  }

  .fill-shimmer {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(255, 255, 255, 0.2) 50%,
      transparent 100%
    );
    animation: shimmer 2.5s ease-in-out infinite;
  }

  @keyframes shimmer {
    0% {
      transform: translateX(-100%);
      opacity: 0.6;
    }
    50% {
      opacity: 1;
    }
    100% {
      transform: translateX(100%);
      opacity: 0.6;
    }
  }

  /* Indeterminate styles */
  .progress-indeterminate {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .bar {
    position: absolute;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--accent-color) 0%, var(--accent2-color) 100%);
  }

  .bar-1 {
    animation: indeterminate1 2s ease-in-out infinite;
  }

  .bar-2 {
    animation: indeterminate2 2s ease-in-out infinite;
    animation-delay: 0.5s;
  }

  @keyframes indeterminate1 {
    0% {
      left: -30%;
      width: 30%;
    }
    50% {
      left: 50%;
      width: 40%;
    }
    100% {
      left: 100%;
      width: 30%;
    }
  }

  @keyframes indeterminate2 {
    0% {
      left: -30%;
      width: 20%;
    }
    50% {
      left: 60%;
      width: 30%;
    }
    100% {
      left: 100%;
      width: 20%;
    }
  }

  .progress-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    min-width: 36px;
    text-align: right;
  }
</style>
