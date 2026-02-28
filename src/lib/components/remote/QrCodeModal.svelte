<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { remoteAccessService } from '@/lib/services/remoteAccessService';
  import { remoteAccessStore } from '@/lib/stores/remoteAccessStore';
  import { toastStore } from '@/lib/stores/toastStore';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let isVisible = $state(false);
  let qrDataUri = $state<string | null>(null);
  let isLoading = $state(true);
  let copied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout> | null = null;
  let dotCount = $state(1);
  let dotInterval: ReturnType<typeof setInterval> | null = null;

  const tunnelUrl = $derived($remoteAccessStore.tunnelUrl);
  const generatingText = $derived('Generating' + '.'.repeat(dotCount));

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  }

  function handleClose() {
    isVisible = false;
    setTimeout(() => {
      onClose();
    }, 200);
  }

  async function handleCopy() {
    if (!tunnelUrl) return;
    try {
      await navigator.clipboard.writeText(tunnelUrl);
      copied = true;
      if (copyTimeout) clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => {
        copied = false;
      }, 2000);
    } catch {
      toastStore.error('Failed to copy URL');
    }
  }

  let prevUrl = $state<string | null>(null);

  async function loadQrCode(url: string) {
    isLoading = true;
    try {
      qrDataUri = await remoteAccessService.generateQrCode($remoteAccessStore.port, url);
    } catch {
      qrDataUri = null;
    } finally {
      isLoading = false;
    }
  }

  // Generate QR code when tunnel URL becomes available or changes
  $effect(() => {
    const currentUrl = $remoteAccessStore.tunnelUrl;
    if (currentUrl && currentUrl !== prevUrl) {
      prevUrl = currentUrl;
      loadQrCode(currentUrl);
    }
  });

  onMount(() => {
    requestAnimationFrame(() => {
      isVisible = true;
    });
    document.addEventListener('keydown', handleKeyDown);
    dotInterval = setInterval(() => {
      dotCount = (dotCount % 3) + 1;
    }, 500);
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown);
    if (copyTimeout) clearTimeout(copyTimeout);
    if (dotInterval) clearInterval(dotInterval);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" class:visible={isVisible} onclick={handleBackdropClick}>
  <div
    class="modal-panel"
    class:visible={isVisible}
    role="dialog"
    aria-modal="true"
    aria-labelledby="qr-modal-title"
  >
    <div class="modal-glow"></div>
    <div class="modal-content">
      <!-- Header -->
      <div class="header">
        <div class="header-icon">
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="3" y="3" width="7" height="7"></rect>
            <rect x="14" y="3" width="7" height="7"></rect>
            <rect x="3" y="14" width="7" height="7"></rect>
            <rect x="14" y="14" width="3" height="3"></rect>
            <line x1="21" y1="14" x2="21" y2="14.01"></line>
            <line x1="21" y1="21" x2="21" y2="21.01"></line>
          </svg>
        </div>
        <h2 id="qr-modal-title">Remote Access</h2>
        <button class="close-btn" onclick={handleClose} aria-label="Close">
          <svg
            width="16"
            height="16"
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

      <!-- Body -->
      <div class="body">
        <div class="qr-container">
          {#if isLoading}
            <div class="qr-loading">
              <div class="spinner"></div>
            </div>
          {:else if qrDataUri}
            <img class="qr-image" src={qrDataUri} alt="QR Code for remote access" />
          {:else}
            <div class="qr-error">
              <span>Failed to generate QR code</span>
            </div>
          {/if}
        </div>

        <div class="url-section">
          <span class="url-label">Remote URL</span>
          <div class="url-row">
            {#if tunnelUrl}
              <code class="url-text">{tunnelUrl}</code>
            {:else}
              <span class="url-generating">{generatingText}</span>
            {/if}
            <button
              class="copy-btn"
              class:copied
              onclick={handleCopy}
              disabled={!tunnelUrl}
              aria-label={copied ? 'Copied' : 'Copy URL'}
            >
              {#if copied}
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
                  <polyline points="20 6 9 17 4 12"></polyline>
                </svg>
              {:else}
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
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                </svg>
              {/if}
            </button>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="footer">
        <div class="brand">
          <span class="brand-text">kiri</span>
          <span class="brand-kanji">{'\u{9727}'}</span>
        </div>
        <span class="hint">
          <kbd>Esc</kbd>
          <span>close</span>
        </span>
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .backdrop.visible {
    opacity: 1;
  }

  .modal-panel {
    position: relative;
    max-width: 400px;
    width: 90%;
    transform: translateY(20px) scale(0.95);
    opacity: 0;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .modal-panel.visible {
    transform: translateY(0) scale(1);
    opacity: 1;
  }

  .modal-glow {
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, var(--accent-color), var(--accent2-color));
    border-radius: calc(var(--radius-xl) + 2px);
    opacity: 0.06;
    filter: blur(5px);
  }

  .modal-content {
    position: relative;
    background: var(--bg-glass);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-xl);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .modal-content::before {
    content: '';
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent-color), transparent);
    opacity: 0.6;
    z-index: 1;
  }

  /* Header */
  .header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--border-color);
  }

  .header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-color);
  }

  .header h2 {
    flex: 1;
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--font-sans);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .close-btn:hover {
    background: var(--bg-tertiary);
    color: var(--accent-color);
  }

  .close-btn svg {
    transition: transform var(--transition-fast);
  }

  .close-btn:hover svg {
    transform: scale(1.1);
  }

  .close-btn:active {
    transform: scale(0.95);
    transition: transform 100ms ease;
  }

  /* Body */
  .body {
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-5);
  }

  .qr-container {
    width: 200px;
    height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: white;
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .qr-image {
    width: 100%;
    height: 100%;
    object-fit: contain;
    padding: 8px;
  }

  .qr-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 2px solid rgba(125, 211, 252, 0.2);
    border-top-color: var(--accent-color);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .qr-error {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    font-size: 12px;
    color: var(--text-muted);
    text-align: center;
    padding: var(--space-3);
  }

  /* URL Section */
  .url-section {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .url-label {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    color: var(--text-muted);
    letter-spacing: 0.08em;
  }

  .url-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: var(--space-2) var(--space-3);
    overflow: hidden;
  }

  .url-text {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent-color);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .url-generating {
    flex: 1;
    font-size: 12px;
    color: var(--text-muted);
    font-style: italic;
  }

  .copy-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: all var(--transition-fast);
  }

  .copy-btn:hover {
    color: var(--accent-color);
    background: rgba(125, 211, 252, 0.08);
    border-color: var(--border-color);
  }

  .copy-btn.copied {
    color: #4ade80;
  }

  /* Footer */
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--border-color);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    opacity: 0.4;
  }

  .brand-text {
    font-size: 11px;
    font-family: var(--font-display);
    color: var(--text-muted);
  }

  .brand-kanji {
    font-size: 10px;
    color: var(--accent-color);
    opacity: 0.5;
  }

  .hint {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 11px;
    color: var(--text-muted);
  }

  .hint kbd {
    padding: 2px 6px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
  }
</style>
