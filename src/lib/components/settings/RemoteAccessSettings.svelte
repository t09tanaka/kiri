<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    loadRemoteAccessSettings,
    saveRemoteAccessSettings,
  } from '@/lib/services/persistenceService';
  import { remoteAccessService } from '@/lib/services/remoteAccessService';
  import { remoteAccessStore, isRemoteActive } from '@/lib/stores/remoteAccessStore';
  import { remoteAccessViewStore } from '@/lib/stores/remoteAccessViewStore';
  import { toastStore } from '@/lib/stores/toastStore';
  import { toggleRemoteAccess } from '@/lib/utils/remoteAccessToggle';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let isVisible = $state(false);

  // Settings state
  let portInput = $state('9876');
  let tunnelTokenInput = $state('');
  let showTunnelToken = $state(false);
  let isSaving = $state(false);
  let cloudflaredAvailable = $state(true);
  let isTogglingRemote = $state(false);
  let remoteError = $state<string | null>(null);
  let copied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout> | null = null;

  const remoteUrl = $derived(
    $remoteAccessStore.tunnelUrl ?? `http://localhost:${$remoteAccessStore.port}`
  );

  async function handleToggle() {
    if (isTogglingRemote) return;
    await toggleRemoteAccess({
      onToggling: (v) => (isTogglingRemote = v),
      onError: (msg) => (remoteError = msg || null),
      onServerStarted: () => remoteAccessViewStore.openQrModal(),
    });
  }

  async function handleCopyUrl() {
    try {
      await navigator.clipboard.writeText(remoteUrl);
      copied = true;
      if (copyTimeout) clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => {
        copied = false;
      }, 2000);
    } catch {
      toastStore.error('Failed to copy URL');
    }
  }

  function handleOpenQr() {
    remoteAccessViewStore.openQrModal();
  }

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

  async function handleClose() {
    // Auto-save on close
    await saveCurrentSettings();
    isVisible = false;
    setTimeout(() => {
      onClose();
    }, 200);
  }

  async function saveCurrentSettings() {
    if (isSaving) return;
    isSaving = true;
    try {
      const settings = await loadRemoteAccessSettings();
      const port = parseInt(portInput, 10);
      if (!isNaN(port) && port >= 1024 && port <= 65535) {
        settings.port = port;
      }
      settings.tunnelToken = tunnelTokenInput.trim() || null;
      await saveRemoteAccessSettings(settings);
    } catch (error) {
      toastStore.error('Failed to save settings: ' + String(error));
    } finally {
      isSaving = false;
    }
  }

  function handlePortChange(e: Event) {
    const target = e.target as HTMLInputElement;
    portInput = target.value;
  }

  function handleTunnelTokenChange(e: Event) {
    const target = e.target as HTMLInputElement;
    tunnelTokenInput = target.value;
  }

  onMount(async () => {
    requestAnimationFrame(() => {
      isVisible = true;
    });

    document.addEventListener('keydown', handleKeyDown);

    try {
      const loaded = await loadRemoteAccessSettings();
      portInput = String(loaded.port);
      tunnelTokenInput = loaded.tunnelToken ?? '';
    } catch {
      toastStore.error('Failed to load remote access settings');
    }

    try {
      cloudflaredAvailable = await remoteAccessService.isCloudflaredAvailable();
    } catch {
      cloudflaredAvailable = false;
    }
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown);
    if (copyTimeout) clearTimeout(copyTimeout);
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
    aria-labelledby="remote-access-title"
  >
    <div class="panel-content">
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
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
            <line x1="8" y1="21" x2="16" y2="21"></line>
            <line x1="12" y1="17" x2="12" y2="21"></line>
          </svg>
        </div>
        <h2 id="remote-access-title">Remote Access</h2>
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

      <!-- Content -->
      <div class="content">
        <!-- ON/OFF Toggle -->
        <div class="toggle-row">
          <div class="toggle-label">
            <span class="label-text">Remote Access</span>
            <span class="toggle-status" class:active={$isRemoteActive}>
              {$isRemoteActive ? 'ON' : 'OFF'}
            </span>
          </div>
          <button
            class="remote-lightswitch"
            class:active={$isRemoteActive}
            onclick={handleToggle}
            disabled={isTogglingRemote}
            aria-label={$isRemoteActive ? 'Stop remote access' : 'Start remote access'}
          >
            <span class="lightswitch-track">
              <span class="lightswitch-thumb"></span>
            </span>
          </button>
        </div>

        {#if remoteError}
          <p class="remote-error">{remoteError}</p>
        {/if}

        <!-- Remote URL Section (visible when active) -->
        {#if $isRemoteActive}
          <div class="url-section">
            <span class="url-label">Remote URL</span>
            <div class="url-row">
              <code class="url-text">{remoteUrl}</code>
              <button
                class="url-action-btn"
                onclick={handleOpenQr}
                aria-label="Show QR code"
                title="Show QR code"
              >
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
                  <rect x="3" y="3" width="7" height="7"></rect>
                  <rect x="14" y="3" width="7" height="7"></rect>
                  <rect x="3" y="14" width="7" height="7"></rect>
                  <rect x="14" y="14" width="3" height="3"></rect>
                  <line x1="21" y1="14" x2="21" y2="14.01"></line>
                  <line x1="21" y1="21" x2="21" y2="21.01"></line>
                </svg>
              </button>
              <button
                class="url-action-btn"
                class:copied
                onclick={handleCopyUrl}
                aria-label={copied ? 'Copied' : 'Copy URL'}
                title={copied ? 'Copied!' : 'Copy URL'}
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
        {/if}

        <p class="description">Access kiri from outside your network via Cloudflare Tunnel.</p>

        {#if !cloudflaredAvailable}
          <div class="warning-banner">
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
              <path
                d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
              ></path>
              <line x1="12" y1="9" x2="12" y2="13"></line>
              <line x1="12" y1="17" x2="12.01" y2="17"></line>
            </svg>
            <span>cloudflared is not installed. Run: <code>brew install cloudflared</code></span>
          </div>
        {/if}

        <div class="control-row">
          <div class="control-label">
            <span class="label-text">Tunnel Token</span>
            <span class="optional-badge">optional</span>
          </div>
          <div class="token-input-wrapper">
            <input
              type={showTunnelToken ? 'text' : 'password'}
              class="token-input"
              value={tunnelTokenInput}
              oninput={handleTunnelTokenChange}
              placeholder="Empty = Quick Tunnel"
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
            />
            <button
              class="token-toggle-btn"
              onclick={() => (showTunnelToken = !showTunnelToken)}
              aria-label={showTunnelToken ? 'Hide token' : 'Show token'}
            >
              {#if showTunnelToken}
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
                  <path
                    d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"
                  ></path>
                  <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"
                  ></path>
                  <line x1="1" y1="1" x2="23" y2="23"></line>
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
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
                  <circle cx="12" cy="12" r="3"></circle>
                </svg>
              {/if}
            </button>
          </div>
        </div>

        <div class="control-row">
          <div class="control-label">
            <span class="label-text">Port</span>
          </div>
          <input
            type="text"
            class="port-input"
            value={portInput}
            oninput={handlePortChange}
            placeholder="9876"
            spellcheck="false"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
          />
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
          <span>to close</span>
        </span>
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .backdrop.visible {
    opacity: 1;
  }

  .modal-panel {
    position: relative;
    max-width: 480px;
    width: 90%;
    max-height: 85vh;
    transform: translateY(20px) scale(0.95);
    opacity: 0;
    transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .modal-panel.visible {
    transform: translateY(0) scale(1);
    opacity: 1;
  }

  .panel-content {
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

  .panel-content::before {
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

  /* Content */
  .content {
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .description {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .warning-banner {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: rgba(252, 211, 77, 0.08);
    border: 1px solid rgba(252, 211, 77, 0.2);
    border-radius: var(--radius-sm);
    font-size: 12px;
    color: var(--accent3-color);
    line-height: 1.4;
  }

  .warning-banner svg {
    flex-shrink: 0;
  }

  .warning-banner code {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 1px 4px;
    background: rgba(252, 211, 77, 0.1);
    border-radius: 3px;
  }

  /* Control Row */
  .control-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .control-label {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-shrink: 0;
  }

  .label-text {
    font-size: 13px;
    color: var(--text-primary);
  }

  .optional-badge {
    font-size: 10px;
    color: var(--text-muted);
    padding: 1px 6px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    opacity: 0.7;
  }

  /* Port Input */
  .port-input {
    width: 90px;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono);
    text-align: right;
    outline: none;
    transition: border-color var(--transition-fast);
  }

  .port-input:focus {
    border-color: var(--accent-color);
  }

  /* Token Input */
  .token-input-wrapper {
    display: flex;
    align-items: center;
    gap: 0;
    flex: 1;
    max-width: 280px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    overflow: hidden;
    transition: border-color var(--transition-fast);
  }

  .token-input-wrapper:focus-within {
    border-color: var(--accent-color);
  }

  .token-input {
    flex: 1;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono);
    outline: none;
    min-width: 0;
  }

  .token-toggle-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: color var(--transition-fast);
  }

  .token-toggle-btn:hover {
    color: var(--text-secondary);
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

  /* Toggle Row */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .toggle-status {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .toggle-status.active {
    background: rgba(125, 211, 252, 0.15);
    border-color: rgba(125, 211, 252, 0.3);
    color: var(--accent-color);
  }

  .remote-lightswitch {
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    flex-shrink: 0;
  }

  .remote-lightswitch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .lightswitch-track {
    display: block;
    width: 36px;
    height: 20px;
    border-radius: 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    position: relative;
    transition: all var(--transition-fast);
  }

  .remote-lightswitch.active .lightswitch-track {
    background: rgba(125, 211, 252, 0.2);
    border-color: var(--accent-color);
  }

  .lightswitch-thumb {
    display: block;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--text-muted);
    position: absolute;
    top: 2px;
    left: 2px;
    transition: all var(--transition-fast);
  }

  .remote-lightswitch.active .lightswitch-thumb {
    left: 18px;
    background: var(--accent-color);
  }

  .remote-error {
    margin: 0;
    font-size: 11px;
    color: var(--accent3-color);
    opacity: 0.85;
  }

  /* URL Section */
  .url-section {
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

  .url-action-btn {
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

  .url-action-btn:hover {
    color: var(--accent-color);
    background: rgba(125, 211, 252, 0.08);
    border-color: var(--border-color);
  }

  .url-action-btn.copied {
    color: #4ade80;
  }
</style>
