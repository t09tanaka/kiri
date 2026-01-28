<script lang="ts">
  import { confirmDialogStore } from '@/lib/stores/confirmDialogStore';
  import { onMount, onDestroy } from 'svelte';

  let mounted = $state(false);
  let confirmButtonRef: HTMLButtonElement | null = $state(null);

  const options = $derived($confirmDialogStore.options);
  const isOpen = $derived($confirmDialogStore.isOpen);

  function handleConfirm() {
    confirmDialogStore.handleConfirm();
  }

  function handleCancel() {
    confirmDialogStore.handleCancel();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!isOpen) return;

    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      handleCancel();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      handleConfirm();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleCancel();
    }
  }

  $effect(() => {
    if (isOpen && confirmButtonRef) {
      confirmButtonRef.focus();
    }
  });

  onMount(() => {
    mounted = true;
    document.addEventListener('keydown', handleKeyDown, true);
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
  });

  // Icon type based on dialog kind
  const iconType = $derived(options?.kind ?? 'info');
</script>

{#if isOpen && options}
  <div
    class="dialog-backdrop"
    class:mounted
    onclick={handleBackdropClick}
    onkeydown={() => {}}
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-dialog-title"
    tabindex="-1"
  >
    <div
      class="dialog-container"
      class:warning={iconType === 'warning'}
      class:error={iconType === 'error'}
    >
      <!-- Ambient glow effect -->
      <div class="dialog-ambient"></div>

      <div class="dialog-content">
        <!-- Icon area with subtle animation -->
        <div
          class="dialog-icon-area"
          class:warning={iconType === 'warning'}
          class:error={iconType === 'error'}
        >
          <div class="icon-container">
            <div class="icon-ring"></div>
            {#if iconType === 'warning'}
              <svg
                class="icon"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M12 9v4"></path>
                <path d="M12 17h.01"></path>
                <path
                  d="M10.363 3.591l-8.106 13.534a1.914 1.914 0 0 0 1.636 2.871h16.214a1.914 1.914 0 0 0 1.636-2.87L13.637 3.59a1.914 1.914 0 0 0-3.274 0z"
                ></path>
              </svg>
            {:else if iconType === 'error'}
              <svg
                class="icon"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <circle cx="12" cy="12" r="10"></circle>
                <path d="M15 9l-6 6"></path>
                <path d="M9 9l6 6"></path>
              </svg>
            {:else}
              <svg
                class="icon"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <circle cx="12" cy="12" r="10"></circle>
                <path d="M12 16v-4"></path>
                <path d="M12 8h.01"></path>
              </svg>
            {/if}
          </div>
        </div>

        <!-- Text content -->
        <div class="dialog-text">
          <h2 id="confirm-dialog-title" class="dialog-title">{options.title}</h2>
          <p class="dialog-message">{options.message}</p>
        </div>

        <!-- Action buttons -->
        <div class="dialog-actions">
          <button class="btn btn-ghost" onclick={handleCancel}>
            <span class="btn-label">{options.cancelLabel}</span>
            <span class="btn-hint">esc</span>
          </button>
          <button
            class="btn btn-confirm"
            class:warning={iconType === 'warning'}
            class:error={iconType === 'error'}
            onclick={handleConfirm}
            bind:this={confirmButtonRef}
          >
            <span class="btn-label">{options.confirmLabel}</span>
            <span class="btn-hint">↵</span>
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(8, 12, 16, 0.75);
    backdrop-filter: blur(8px) saturate(120%);
    -webkit-backdrop-filter: blur(8px) saturate(120%);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
    opacity: 0;
    transition: opacity 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .dialog-backdrop.mounted {
    opacity: 1;
  }

  .dialog-container {
    position: relative;
    width: 90%;
    max-width: 380px;
    transform: translateY(12px) scale(0.96);
    opacity: 0;
    animation: dialogSlideIn 0.35s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  @keyframes dialogSlideIn {
    to {
      transform: translateY(0) scale(1);
      opacity: 1;
    }
  }

  /* Ambient glow - subtle, atmospheric */
  .dialog-ambient {
    position: absolute;
    inset: -20px;
    background: radial-gradient(
      ellipse 60% 50% at 50% 0%,
      rgba(125, 211, 252, 0.12) 0%,
      transparent 60%
    );
    border-radius: 50%;
    pointer-events: none;
    z-index: -1;
    animation: ambientPulse 4s ease-in-out infinite;
  }

  .dialog-container.warning .dialog-ambient {
    background: radial-gradient(
      ellipse 60% 50% at 50% 0%,
      rgba(251, 191, 36, 0.1) 0%,
      transparent 60%
    );
  }

  .dialog-container.error .dialog-ambient {
    background: radial-gradient(
      ellipse 60% 50% at 50% 0%,
      rgba(248, 113, 113, 0.1) 0%,
      transparent 60%
    );
  }

  @keyframes ambientPulse {
    0%,
    100% {
      opacity: 0.8;
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(1.05);
    }
  }

  .dialog-content {
    background: linear-gradient(180deg, rgba(26, 32, 41, 0.95) 0%, rgba(19, 25, 32, 0.98) 100%);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(125, 211, 252, 0.08);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.3),
      0 8px 40px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  /* Icon area - centered, prominent */
  .dialog-icon-area {
    display: flex;
    justify-content: center;
    padding: var(--space-5) var(--space-4) var(--space-3);
  }

  .icon-container {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 52px;
    height: 52px;
  }

  .icon-ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent-color), var(--accent2-color));
    opacity: 0.08;
    animation: ringPulse 3s ease-in-out infinite;
  }

  .dialog-icon-area.warning .icon-ring {
    background: linear-gradient(135deg, #fbbf24, #f59e0b);
    opacity: 0.12;
  }

  .dialog-icon-area.error .icon-ring {
    background: linear-gradient(135deg, #f87171, #ef4444);
    opacity: 0.12;
  }

  @keyframes ringPulse {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.08;
    }
    50% {
      transform: scale(1.15);
      opacity: 0.04;
    }
  }

  .icon {
    position: relative;
    z-index: 1;
    color: var(--accent-color);
    filter: drop-shadow(0 2px 8px rgba(125, 211, 252, 0.3));
    animation: iconFloat 3s ease-in-out infinite;
  }

  .dialog-icon-area.warning .icon {
    color: #fbbf24;
    filter: drop-shadow(0 2px 8px rgba(251, 191, 36, 0.3));
  }

  .dialog-icon-area.error .icon {
    color: #f87171;
    filter: drop-shadow(0 2px 8px rgba(248, 113, 113, 0.3));
  }

  @keyframes iconFloat {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-2px);
    }
  }

  /* Text content - centered, clean typography */
  .dialog-text {
    text-align: center;
    padding: 0 var(--space-5) var(--space-4);
  }

  .dialog-title {
    margin: 0 0 var(--space-2);
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 500;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .dialog-message {
    margin: 0;
    font-size: 13px;
    line-height: 1.7;
    color: var(--text-secondary);
    max-width: 280px;
    margin: 0 auto;
  }

  /* Action buttons - clean, minimal */
  .dialog-actions {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4) var(--space-4);
    justify-content: center;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    min-width: 110px;
    padding: var(--space-2) var(--space-4);
    font-family: var(--font-sans);
    font-size: 13px;
    font-weight: 500;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    overflow: hidden;
  }

  .btn-label {
    position: relative;
    z-index: 1;
  }

  .btn-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 400;
    opacity: 0.4;
    margin-left: var(--space-1);
    text-transform: none;
  }

  /* Ghost button (cancel) */
  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
  }

  .btn-ghost:hover {
    background: rgba(125, 211, 252, 0.05);
    color: var(--text-primary);
    border-color: var(--border-glow);
  }

  .btn-ghost:active {
    transform: scale(0.97);
  }

  /* Confirm button */
  .btn-confirm {
    background: linear-gradient(
      135deg,
      rgba(125, 211, 252, 0.15) 0%,
      rgba(196, 181, 253, 0.15) 100%
    );
    color: var(--accent-color);
    border: 1px solid rgba(125, 211, 252, 0.2);
  }

  .btn-confirm::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(125, 211, 252, 0.2) 0%, rgba(196, 181, 253, 0.2) 100%);
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .btn-confirm:hover::before {
    opacity: 1;
  }

  .btn-confirm:hover {
    border-color: rgba(125, 211, 252, 0.35);
    box-shadow: 0 0 20px rgba(125, 211, 252, 0.15);
  }

  .btn-confirm:active {
    transform: scale(0.97);
  }

  /* Warning variant */
  .btn-confirm.warning {
    background: linear-gradient(135deg, rgba(251, 191, 36, 0.12) 0%, rgba(245, 158, 11, 0.12) 100%);
    color: #fbbf24;
    border: 1px solid rgba(251, 191, 36, 0.2);
  }

  .btn-confirm.warning::before {
    background: linear-gradient(135deg, rgba(251, 191, 36, 0.2) 0%, rgba(245, 158, 11, 0.2) 100%);
  }

  .btn-confirm.warning:hover {
    border-color: rgba(251, 191, 36, 0.35);
    box-shadow: 0 0 20px rgba(251, 191, 36, 0.12);
  }

  /* Error variant */
  .btn-confirm.error {
    background: linear-gradient(135deg, rgba(248, 113, 113, 0.12) 0%, rgba(239, 68, 68, 0.12) 100%);
    color: #f87171;
    border: 1px solid rgba(248, 113, 113, 0.2);
  }

  .btn-confirm.error::before {
    background: linear-gradient(135deg, rgba(248, 113, 113, 0.2) 0%, rgba(239, 68, 68, 0.2) 100%);
  }

  .btn-confirm.error:hover {
    border-color: rgba(248, 113, 113, 0.35);
    box-shadow: 0 0 20px rgba(248, 113, 113, 0.12);
  }

  /* Focus states */
  .btn:focus {
    outline: none;
  }

  .btn:focus-visible {
    box-shadow:
      0 0 0 2px var(--bg-primary),
      0 0 0 4px var(--accent-color);
  }

  .btn-confirm.warning:focus-visible {
    box-shadow:
      0 0 0 2px var(--bg-primary),
      0 0 0 4px #fbbf24;
  }

  .btn-confirm.error:focus-visible {
    box-shadow:
      0 0 0 2px var(--bg-primary),
      0 0 0 4px #f87171;
  }
</style>
