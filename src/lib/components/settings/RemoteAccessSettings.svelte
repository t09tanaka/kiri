<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { remoteAccessService } from '@/lib/services/remoteAccessService';
  import { remoteAccessStore } from '@/lib/stores/remoteAccessStore';
  import {
    loadRemoteAccessSettings,
    saveRemoteAccessSettings,
    type RemoteAccessSettings,
  } from '@/lib/services/persistenceService';
  import { toastStore } from '@/lib/stores/toastStore';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let isVisible = $state(false);

  // Store state
  const store = $derived($remoteAccessStore);

  // Settings state
  let settings = $state<RemoteAccessSettings | null>(null);
  let portInput = $state('9876');

  // QR Code state
  let qrCodeData = $state<string | null>(null);
  let isGeneratingQr = $state(false);

  // Tunnel token visibility
  let showTunnelToken = $state(false);
  let tunnelTokenInput = $state('');
  let tunnelUrlInput = $state('');

  // Loading states
  let isTogglingServer = $state(false);
  let isTogglingTunnel = $state(false);

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

  async function toggleServer() {
    if (!settings || isTogglingServer) return;
    isTogglingServer = true;

    try {
      if (store.serverRunning) {
        await remoteAccessService.stopServer();
        remoteAccessStore.setServerRunning(false);
        settings.enabled = false;
        toastStore.info('Remote access server stopped');
      } else {
        const port = parseInt(portInput, 10);
        if (isNaN(port) || port < 1024 || port > 65535) {
          toastStore.error('Port must be between 1024 and 65535');
          isTogglingServer = false;
          return;
        }
        await remoteAccessService.startServer(port);
        remoteAccessStore.setServerRunning(true);
        remoteAccessStore.setPort(port);
        remoteAccessStore.setHasToken(true);
        settings.enabled = true;
        settings.port = port;
        toastStore.success('Remote access server started on port ' + port);
      }
      await saveRemoteAccessSettings(settings);
    } catch (error) {
      toastStore.error('Failed to toggle server: ' + String(error));
    } finally {
      isTogglingServer = false;
    }
  }

  async function handleGenerateQr() {
    if (isGeneratingQr) return;
    isGeneratingQr = true;

    try {
      const port = parseInt(portInput, 10);
      const tunnelUrl = store.tunnelRunning
        ? store.tunnelUrl || tunnelUrlInput.trim() || undefined
        : undefined;
      qrCodeData = await remoteAccessService.generateQrCode(port, tunnelUrl);
    } catch (error) {
      toastStore.error('Failed to generate QR code: ' + String(error));
    } finally {
      isGeneratingQr = false;
    }
  }

  async function toggleTunnel() {
    if (!settings || isTogglingTunnel) return;
    isTogglingTunnel = true;

    try {
      if (store.tunnelRunning) {
        await remoteAccessService.stopTunnel();
        remoteAccessStore.setTunnelRunning(false);
        toastStore.info('Cloudflare tunnel stopped');
      } else {
        const token = tunnelTokenInput.trim() || null;
        const port = parseInt(portInput, 10);
        const tunnelUrl = await remoteAccessService.startTunnel(token, port);
        remoteAccessStore.setTunnelRunning(true, tunnelUrl ?? undefined);
        if (token) {
          settings.tunnelToken = token;
          settings.tunnelUrl = tunnelUrlInput.trim() || null;
        }
        toastStore.success('Cloudflare tunnel started');
      }
      await saveRemoteAccessSettings(settings);
    } catch (error) {
      toastStore.error('Failed to toggle tunnel: ' + String(error));
    } finally {
      isTogglingTunnel = false;
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

  function handleTunnelUrlChange(e: Event) {
    const target = e.target as HTMLInputElement;
    tunnelUrlInput = target.value;
  }

  onMount(async () => {
    // Trigger enter animation on next frame
    requestAnimationFrame(() => {
      isVisible = true;
    });

    document.addEventListener('keydown', handleKeyDown);

    try {
      const loaded = await loadRemoteAccessSettings();
      settings = loaded;
      portInput = String(loaded.port);
      tunnelTokenInput = loaded.tunnelToken ?? '';
      tunnelUrlInput = loaded.tunnelUrl ?? '';

      // Sync store with persisted settings
      remoteAccessStore.setPort(loaded.port);
      if (loaded.authToken) {
        remoteAccessStore.setHasToken(true);
      }

      // Check actual server status
      try {
        const running = await remoteAccessService.isRunning();
        remoteAccessStore.setServerRunning(running);
      } catch {
        // Server status check may fail if backend command not available yet
      }
    } catch {
      toastStore.error('Failed to load remote access settings');
    }
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown);
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
        <!-- Section 1: Server Control -->
        <div class="section">
          <div class="section-header">
            <h3 class="section-title">Server Control</h3>
            <span class="section-description"> Start or stop the remote access HTTP server. </span>
          </div>

          <div class="control-row">
            <div class="control-label">
              <span class="label-text">Remote Access</span>
              <span class="status-indicator" class:active={store.serverRunning}>
                <span class="status-dot"></span>
                <span class="status-text">{store.serverRunning ? 'Running' : 'Stopped'}</span>
              </span>
            </div>
            <button
              class="toggle-btn"
              class:active={store.serverRunning}
              onclick={toggleServer}
              disabled={isTogglingServer}
              aria-label={store.serverRunning ? 'Stop server' : 'Start server'}
            >
              <span class="toggle-track">
                <span class="toggle-thumb"></span>
              </span>
            </button>
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
              disabled={store.serverRunning}
              placeholder="9876"
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
            />
          </div>

          <!-- QR Code in Server Control -->
          {#if store.serverRunning}
            <div class="qr-section">
              <button class="action-btn" onclick={handleGenerateQr} disabled={isGeneratingQr}>
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
                  <rect x="14" y="14" width="7" height="7"></rect>
                </svg>
                {isGeneratingQr ? 'Generating...' : 'Show QR Code'}
              </button>

              {#if qrCodeData}
                <div class="qr-container">
                  <img src={qrCodeData} alt="QR Code for remote access" class="qr-image" />
                  <span class="qr-hint">Scan with your mobile device to connect</span>
                </div>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Section 2: Cloudflare Tunnel -->
        <div class="section">
          <div class="section-header">
            <h3 class="section-title">Cloudflare Tunnel</h3>
            <span class="section-description">
              Expose the server via Cloudflare Tunnel. Leave token empty for a Quick Tunnel.
            </span>
          </div>

          <div class="control-row">
            <div class="control-label">
              <span class="label-text">Tunnel</span>
              <span class="status-indicator" class:active={store.tunnelRunning}>
                <span class="status-dot"></span>
                <span class="status-text">{store.tunnelRunning ? 'Connected' : 'Disconnected'}</span
                >
              </span>
            </div>
            <button
              class="toggle-btn"
              class:active={store.tunnelRunning}
              onclick={toggleTunnel}
              disabled={isTogglingTunnel || !store.serverRunning}
              aria-label={store.tunnelRunning ? 'Stop tunnel' : 'Start tunnel'}
            >
              <span class="toggle-track">
                <span class="toggle-thumb"></span>
              </span>
            </button>
          </div>

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
                disabled={store.tunnelRunning}
                placeholder="Quick Tunnel (no token)"
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

          {#if tunnelTokenInput.trim()}
            <div class="control-row">
              <div class="control-label">
                <span class="label-text">Tunnel URL</span>
              </div>
              <input
                type="text"
                class="tunnel-url-input"
                value={tunnelUrlInput}
                oninput={handleTunnelUrlChange}
                disabled={store.tunnelRunning}
                placeholder="https://your-tunnel.example.com"
                spellcheck="false"
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
              />
            </div>
          {/if}

          {#if store.tunnelUrl}
            <div class="tunnel-url">
              <span class="tunnel-url-label">Active URL:</span>
              <code class="tunnel-url-value">{store.tunnelUrl}</code>
            </div>
          {/if}
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
    max-width: 560px;
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
    overflow-y: auto;
    max-height: calc(85vh - 130px);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  /* Section */
  .section {
    padding: var(--space-4) 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .section + .section {
    border-top: 1px solid var(--border-color);
  }

  .section-header {
    margin-bottom: var(--space-1);
  }

  .section-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 var(--space-1) 0;
  }

  .section-description {
    font-size: 12px;
    color: var(--text-muted);
  }

  /* Control Row */
  .control-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) 0;
  }

  .control-label {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex: 1;
    min-width: 0;
  }

  .label-text {
    font-size: 13px;
    color: var(--text-primary);
  }

  /* Optional Badge */
  .optional-badge {
    font-size: 10px;
    color: var(--text-muted);
    padding: 1px 6px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    opacity: 0.7;
  }

  /* Status Indicator */
  .status-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--git-deleted);
    transition: background var(--transition-normal);
    box-shadow: 0 0 6px rgba(248, 113, 113, 0.4);
  }

  .status-indicator.active .status-dot {
    background: var(--git-added);
    box-shadow: 0 0 6px rgba(74, 222, 128, 0.4);
  }

  .status-text {
    font-size: 11px;
    color: var(--text-muted);
  }

  /* Toggle Button */
  .toggle-btn {
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    flex-shrink: 0;
  }

  .toggle-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .toggle-track {
    display: block;
    width: 40px;
    height: 22px;
    border-radius: 11px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    position: relative;
    transition: all var(--transition-fast);
  }

  .toggle-btn.active .toggle-track {
    background: rgba(125, 211, 252, 0.2);
    border-color: var(--accent-color);
  }

  .toggle-thumb {
    display: block;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-muted);
    position: absolute;
    top: 2px;
    left: 2px;
    transition: all var(--transition-fast);
  }

  .toggle-btn.active .toggle-thumb {
    left: 20px;
    background: var(--accent-color);
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

  .port-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* QR Section */
  .qr-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border-subtle);
  }

  /* Action Button */
  .action-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: rgba(125, 211, 252, 0.1);
    border: 1px solid rgba(125, 211, 252, 0.3);
    border-radius: var(--radius-md);
    color: var(--accent-color);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all var(--transition-fast);
    width: fit-content;
  }

  .action-btn:hover:not(:disabled) {
    background: rgba(125, 211, 252, 0.2);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* QR Code */
  .qr-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-4);
    background: rgba(255, 255, 255, 0.95);
    border-radius: var(--radius-md);
  }

  .qr-image {
    width: 200px;
    height: 200px;
    image-rendering: pixelated;
  }

  .qr-hint {
    font-size: 11px;
    color: var(--bg-primary);
    opacity: 0.7;
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

  .token-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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

  /* Tunnel URL Input */
  .tunnel-url-input {
    flex: 1;
    max-width: 280px;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono);
    outline: none;
    transition: border-color var(--transition-fast);
  }

  .tunnel-url-input:focus {
    border-color: var(--accent-color);
  }

  .tunnel-url-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Tunnel URL Display */
  .tunnel-url {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-tertiary);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
  }

  .tunnel-url-label {
    font-size: 12px;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .tunnel-url-value {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--accent-color);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Footer */
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--border-subtle);
    background: rgba(0, 0, 0, 0.2);
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 4px;
  }

  .brand-text {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
  }

  .brand-kanji {
    font-size: 10px;
    color: var(--text-muted);
    opacity: 0.5;
  }

  .hint {
    font-size: 11px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .hint kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: auto;
    height: 20px;
    padding: 0 6px;
    background: linear-gradient(180deg, var(--bg-tertiary) 0%, var(--bg-secondary) 100%);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-size: 10px;
    font-family: var(--font-mono);
    font-weight: 500;
    color: var(--text-primary);
  }
</style>
