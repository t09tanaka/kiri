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
    <div class="dialog-container" class:warning={options.kind === 'warning'}>
      <div class="dialog-glow"></div>
      <div class="dialog-content">
        <div class="dialog-header">
          {#if options.kind === 'warning'}
            <svg
              class="icon warning"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
              ></path>
              <line x1="12" y1="9" x2="12" y2="13"></line>
              <line x1="12" y1="17" x2="12.01" y2="17"></line>
            </svg>
          {:else if options.kind === 'error'}
            <svg
              class="icon error"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="10"></circle>
              <line x1="15" y1="9" x2="9" y2="15"></line>
              <line x1="9" y1="9" x2="15" y2="15"></line>
            </svg>
          {:else}
            <svg
              class="icon info"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="10"></circle>
              <line x1="12" y1="16" x2="12" y2="12"></line>
              <line x1="12" y1="8" x2="12.01" y2="8"></line>
            </svg>
          {/if}
          <h2 id="confirm-dialog-title" class="dialog-title">{options.title}</h2>
        </div>

        <div class="dialog-body">
          <p class="dialog-message">{options.message}</p>
        </div>

        <div class="dialog-footer">
          <button class="btn btn-secondary" onclick={handleCancel}>
            {options.cancelLabel}
            <kbd>Esc</kbd>
          </button>
          <button
            class="btn btn-primary"
            class:warning={options.kind === 'warning'}
            onclick={handleConfirm}
            bind:this={confirmButtonRef}
          >
            {options.confirmLabel}
            <kbd>Enter</kbd>
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
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .dialog-backdrop.mounted {
    opacity: 1;
  }

  .dialog-container {
    position: relative;
    width: 90%;
    max-width: 420px;
    transform: scale(0.95);
    opacity: 0;
    animation: dialogIn 0.2s ease forwards;
  }

  @keyframes dialogIn {
    to {
      transform: scale(1);
      opacity: 1;
    }
  }

  .dialog-glow {
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, var(--accent-color), var(--accent2-color));
    border-radius: calc(var(--radius-lg) + 2px);
    opacity: 0.08;
    filter: blur(4px);
    z-index: -1;
  }

  .dialog-container.warning .dialog-glow {
    background: linear-gradient(135deg, #f59e0b, #ef4444);
  }

  .dialog-content {
    background: var(--bg-glass);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow-lg);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4);
    background: rgba(0, 0, 0, 0.15);
    border-bottom: 1px solid var(--border-subtle);
  }

  .icon {
    flex-shrink: 0;
  }

  .icon.info {
    color: var(--accent-color);
  }

  .icon.warning {
    color: #f59e0b;
  }

  .icon.error {
    color: #ef4444;
  }

  .dialog-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.01em;
  }

  .dialog-body {
    padding: var(--space-4);
  }

  .dialog-message {
    margin: 0;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-secondary);
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.1);
    border-top: 1px solid var(--border-subtle);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    font-size: 13px;
    font-weight: 500;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn kbd {
    padding: 1px 5px;
    font-size: 10px;
    font-family: var(--font-mono);
    background: rgba(0, 0, 0, 0.2);
    border-radius: var(--radius-sm);
    opacity: 0.6;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
  }

  .btn-secondary:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .btn-primary {
    background: linear-gradient(135deg, var(--accent-color), var(--accent2-color));
    color: var(--bg-primary);
  }

  .btn-primary:hover {
    filter: brightness(1.1);
  }

  .btn-primary.warning {
    background: linear-gradient(135deg, #f59e0b, #ef4444);
  }

  .btn:focus {
    outline: none;
    box-shadow:
      0 0 0 2px var(--bg-primary),
      0 0 0 4px var(--accent-color);
  }

  .btn:active {
    transform: scale(0.98);
  }
</style>
