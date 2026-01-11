<script lang="ts">
  interface Props {
    variant?: 'default' | 'success' | 'warning' | 'error' | 'info' | 'muted';
    size?: 'sm' | 'md';
    glow?: boolean;
    pulse?: boolean;
    children: import('svelte').Snippet;
  }

  let { variant = 'default', size = 'md', glow = false, pulse = false, children }: Props = $props();
</script>

<span
  class="badge"
  class:default={variant === 'default'}
  class:success={variant === 'success'}
  class:warning={variant === 'warning'}
  class:error={variant === 'error'}
  class:info={variant === 'info'}
  class:muted={variant === 'muted'}
  class:sm={size === 'sm'}
  class:md={size === 'md'}
  class:glow
  class:pulse
>
  <span class="badge-content">
    {@render children()}
  </span>
  {#if glow}
    <span class="badge-glow"></span>
  {/if}
</span>

<style>
  .badge {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 500;
    letter-spacing: 0.02em;
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
    overflow: hidden;
  }

  .badge-content {
    position: relative;
    z-index: 1;
  }

  /* Sizes */
  .badge.sm {
    padding: 2px 6px;
    font-size: 10px;
  }

  .badge.md {
    padding: 4px 10px;
    font-size: 11px;
  }

  /* Variants */
  .badge.default {
    background: var(--accent-subtle);
    color: var(--accent-color);
    border: 1px solid var(--accent-glow);
  }

  .badge.success {
    background: rgba(74, 222, 128, 0.1);
    color: var(--git-added);
    border: 1px solid rgba(74, 222, 128, 0.2);
  }

  .badge.warning {
    background: rgba(251, 191, 36, 0.1);
    color: var(--git-modified);
    border: 1px solid rgba(251, 191, 36, 0.2);
  }

  .badge.error {
    background: rgba(248, 113, 113, 0.1);
    color: var(--git-deleted);
    border: 1px solid rgba(248, 113, 113, 0.2);
  }

  .badge.info {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
    border: 1px solid rgba(125, 211, 252, 0.2);
  }

  .badge.muted {
    background: var(--bg-tertiary);
    color: var(--text-muted);
    border: 1px solid var(--border-subtle);
  }

  /* Glow effect */
  .badge-glow {
    position: absolute;
    inset: -1px;
    border-radius: inherit;
    opacity: 0.08;
    filter: blur(3px);
    z-index: 0;
    transition: all var(--transition-fast);
  }

  .badge:hover .badge-glow {
    opacity: 0.15;
    filter: blur(4px);
  }

  .badge.default .badge-glow {
    background: var(--accent-color);
  }

  .badge.success .badge-glow {
    background: var(--git-added);
  }

  .badge.warning .badge-glow {
    background: var(--git-modified);
  }

  .badge.error .badge-glow {
    background: var(--git-deleted);
  }

  .badge.info .badge-glow {
    background: var(--accent-color);
  }

  /* Pulse animation */
  .badge.pulse {
    animation: badgePulse 2.5s ease-in-out infinite;
  }

  @keyframes badgePulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.85;
      transform: scale(1.03);
    }
  }

  /* Hover effects */
  .badge:hover {
    transform: translateY(-1px);
  }

  .badge:active {
    transform: translateY(0) scale(0.97);
    transition: transform 80ms ease;
  }

  .badge.glow:hover .badge-glow {
    opacity: 0.25;
  }
</style>
