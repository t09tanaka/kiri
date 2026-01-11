<script lang="ts">
  import { onMount } from 'svelte';

  interface Props {
    message: string;
    type?: 'info' | 'success' | 'warning' | 'error';
    duration?: number;
    onClose?: () => void;
  }

  let { message, type = 'info', duration = 3000, onClose }: Props = $props();

  let visible = $state(false);
  let exiting = $state(false);

  const icons = {
    info: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>`,
    success: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>`,
    warning: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>`,
    error: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg>`,
  };

  function close() {
    exiting = true;
    setTimeout(() => {
      visible = false;
      onClose?.();
    }, 300);
  }

  onMount(() => {
    // Trigger entrance animation
    requestAnimationFrame(() => {
      visible = true;
    });

    // Auto close
    if (duration > 0) {
      const timer = setTimeout(close, duration);
      return () => clearTimeout(timer);
    }
  });
</script>

<div
  class="toast"
  class:visible
  class:exiting
  class:info={type === 'info'}
  class:success={type === 'success'}
  class:warning={type === 'warning'}
  class:error={type === 'error'}
  role="alert"
>
  <div class="toast-glow"></div>
  <div class="toast-content">
    <span class="toast-icon">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      {@html icons[type]}
    </span>
    <span class="toast-message">{message}</span>
    <button class="toast-close" onclick={close} aria-label="Close">
      <svg
        width="14"
        height="14"
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
  <div class="toast-progress" style="animation-duration: {duration}ms"></div>
</div>

<style>
  .toast {
    position: relative;
    min-width: 280px;
    max-width: 420px;
    background: var(--bg-glass);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    opacity: 0;
    transform: translateX(100%) scale(0.9);
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    box-shadow: var(--shadow-lg);
  }

  .toast.visible {
    opacity: 1;
    transform: translateX(0) scale(1);
  }

  .toast.exiting {
    opacity: 0;
    transform: translateX(50%) scale(0.95);
    transition: all 0.2s ease-out;
  }

  /* Type-specific border colors */
  .toast.info {
    border-color: rgba(125, 211, 252, 0.3);
  }

  .toast.success {
    border-color: rgba(74, 222, 128, 0.3);
  }

  .toast.warning {
    border-color: rgba(251, 191, 36, 0.3);
  }

  .toast.error {
    border-color: rgba(248, 113, 113, 0.3);
  }

  .toast-glow {
    position: absolute;
    inset: -1px;
    border-radius: inherit;
    opacity: 0.1;
    pointer-events: none;
    z-index: -1;
    transition: opacity 0.3s ease;
  }

  .toast:hover .toast-glow {
    opacity: 0.18;
  }

  .toast.info .toast-glow {
    background: linear-gradient(135deg, rgba(125, 211, 252, 0.2), transparent);
  }

  .toast.success .toast-glow {
    background: linear-gradient(135deg, rgba(74, 222, 128, 0.2), transparent);
  }

  .toast.warning .toast-glow {
    background: linear-gradient(135deg, rgba(251, 191, 36, 0.2), transparent);
  }

  .toast.error .toast-glow {
    background: linear-gradient(135deg, rgba(248, 113, 113, 0.2), transparent);
  }

  .toast-content {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
  }

  .toast-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .toast.info .toast-icon {
    color: var(--accent-color);
  }

  .toast.success .toast-icon {
    color: var(--git-added);
  }

  .toast.warning .toast-icon {
    color: #fbbf24;
  }

  .toast.error .toast-icon {
    color: var(--git-deleted);
  }

  .toast-message {
    flex: 1;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.4;
  }

  .toast-close {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .toast-close svg {
    transition: transform var(--transition-fast);
  }

  .toast-close:hover {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-primary);
  }

  .toast-close:hover svg {
    transform: scale(1.1);
  }

  .toast-close:active {
    transform: scale(0.9);
  }

  .toast-progress {
    position: absolute;
    bottom: 0;
    left: 0;
    height: 2px;
    width: 100%;
    transform-origin: left;
    animation: progressShrink linear forwards;
  }

  .toast.info .toast-progress {
    background: linear-gradient(90deg, var(--accent-color), var(--accent2-color));
  }

  .toast.success .toast-progress {
    background: linear-gradient(90deg, var(--git-added), #22c55e);
  }

  .toast.warning .toast-progress {
    background: linear-gradient(90deg, #fbbf24, #f59e0b);
  }

  .toast.error .toast-progress {
    background: linear-gradient(90deg, var(--git-deleted), #ef4444);
  }

  @keyframes progressShrink {
    from {
      transform: scaleX(1);
    }
    to {
      transform: scaleX(0);
    }
  }
</style>
