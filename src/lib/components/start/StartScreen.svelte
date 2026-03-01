<script lang="ts">
  import { dialogService } from '@/lib/services/dialogService';
  import {
    saveSettings,
    STARTUP_COMMANDS,
    type StartupCommand,
  } from '@/lib/services/persistenceService';
  import { remoteAccessService } from '@/lib/services/remoteAccessService';
  import { projectStore, recentProjects, type RecentProject } from '@/lib/stores/projectStore';
  import { remoteAccessStore, isRemoteActive } from '@/lib/stores/remoteAccessStore';
  import { remoteAccessViewStore } from '@/lib/stores/remoteAccessViewStore';
  import { settingsStore, startupCommand } from '@/lib/stores/settingsStore';
  import { tabStore } from '@/lib/stores/tabStore';
  import { toggleRemoteAccess } from '@/lib/utils/remoteAccessToggle';
  import RecentProjectItem from './RecentProjectItem.svelte';
  import { onMount } from 'svelte';

  let mounted = $state(false);
  let isTogglingRemote = $state(false);
  let remoteError = $state<string | null>(null);
  let cloudflaredAvailable = $state(true);

  async function handleOpenDirectory() {
    const selected = await dialogService.openDirectory();

    if (selected) {
      await projectStore.openProject(selected);
      // Open a default terminal tab when opening a new project
      tabStore.addTerminalTab();
    }
  }

  function handleProjectSelect(project: RecentProject) {
    projectStore.openProject(project.path);
    // Open a default terminal tab when opening a new project
    tabStore.addTerminalTab();
  }

  function handleProjectRemove(project: RecentProject) {
    projectStore.removeProject(project.path);
  }

  function handleStartupCommandChange(command: StartupCommand) {
    settingsStore.setStartupCommand(command);
    saveSettings(settingsStore.getStateForPersistence());
  }

  async function handleRemoteToggle() {
    if (isTogglingRemote) return;
    // Check cloudflared availability before attempting to turn on
    if (!$isRemoteActive && !cloudflaredAvailable) {
      remoteError = 'cloudflared is not installed. Run: brew install cloudflared';
      return;
    }
    // Open QR modal immediately for instant feedback (before any async work)
    if (!$isRemoteActive) {
      remoteAccessViewStore.openQrModal();
    }
    const result = await toggleRemoteAccess({
      onToggling: (v) => (isTogglingRemote = v),
      onError: (msg) => {
        remoteError = msg || null;
        // Close QR modal on error (e.g. cloudflared not available)
        if (msg) remoteAccessViewStore.closeQrModal();
      },
    });
    // Close QR modal if toggle failed (result is null when turning ON means error)
    if (!result && !$isRemoteActive) {
      remoteAccessViewStore.closeQrModal();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'o') {
      e.preventDefault();
      handleOpenDirectory();
    }
  }

  onMount(async () => {
    mounted = true;
    try {
      const running = await remoteAccessService.isRunning();
      remoteAccessStore.setServerRunning(running);
    } catch {
      // Backend not ready yet
    }
    try {
      cloudflaredAvailable = await remoteAccessService.isCloudflaredAvailable();
    } catch {
      cloudflaredAvailable = false;
    }
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="start-screen">
  <!-- Misty background layers -->
  <div class="bg-layer bg-base"></div>
  <div class="bg-layer bg-mist mist-1"></div>
  <div class="bg-layer bg-mist mist-2"></div>
  <div class="bg-layer bg-mist mist-3"></div>
  <div class="bg-layer bg-aurora"></div>
  <div class="bg-layer bg-noise"></div>
  <div class="bg-layer bg-vignette"></div>

  <!-- Floating particles -->
  <div class="particles">
    {#each Array(12) as _, i (i)}
      <div class="particle" style="--i: {i}"></div>
    {/each}
  </div>

  <div class="content" class:mounted>
    <header class="header">
      <!-- Minimal elegant logo -->
      <div class="logo-container">
        <div class="logo-mist"></div>
        <svg class="logo" width="48" height="48" viewBox="0 0 48 48" fill="none">
          <defs>
            <linearGradient id="mistGradient" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stop-color="var(--accent-color)" stop-opacity="0.8" />
              <stop offset="100%" stop-color="var(--accent2-color)" stop-opacity="0.6" />
            </linearGradient>
            <filter id="softGlow" x="-50%" y="-50%" width="200%" height="200%">
              <feGaussianBlur stdDeviation="2" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>
          <!-- Soft mountain silhouette with mist -->
          <path
            d="M8 36 L16 22 L20 28 L28 16 L36 26 L40 36"
            stroke="url(#mistGradient)"
            stroke-width="1.5"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
            filter="url(#softGlow)"
          />
          <!-- Mist layers -->
          <path
            d="M4 38 Q12 35 24 37 Q36 35 44 38"
            stroke="var(--accent-color)"
            stroke-width="1"
            fill="none"
            opacity="0.5"
            stroke-linecap="round"
          />
          <path
            d="M6 42 Q18 39 30 41 Q42 39 46 42"
            stroke="var(--accent2-color)"
            stroke-width="1"
            fill="none"
            opacity="0.3"
            stroke-linecap="round"
          />
        </svg>
      </div>

      <h1 class="title">
        <span class="title-main">kiri</span>
        <span class="title-kanji">霧</span>
      </h1>
      <p class="subtitle">Light as mist, powerful as code</p>
    </header>

    <button class="open-button" onclick={handleOpenDirectory}>
      <div class="button-bg"></div>
      <div class="button-content">
        <span class="open-icon">
          <svg
            width="22"
            height="22"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            <line x1="12" y1="11" x2="12" y2="17" />
            <line x1="9" y1="14" x2="15" y2="14" />
          </svg>
        </span>
        <div class="open-text">
          <span class="open-title">Open Directory</span>
          <span class="open-desc">Select a folder to start</span>
        </div>
        <span class="open-shortcut">
          <kbd>⌘</kbd><kbd>O</kbd>
        </span>
      </div>
    </button>

    <div class="startup-command-row">
      <span class="startup-label">
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
          <polyline points="4 17 10 11 4 5"></polyline>
          <line x1="12" y1="19" x2="20" y2="19"></line>
        </svg>
        Startup Command
      </span>
      <div class="segment-control">
        {#each STARTUP_COMMANDS as cmd (cmd.id)}
          <button
            class="segment-option"
            class:active={$startupCommand === cmd.id}
            onclick={() => handleStartupCommandChange(cmd.id)}
          >
            {cmd.label}
          </button>
        {/each}
      </div>
    </div>

    <div class="startup-command-row">
      <span class="startup-label">
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
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
          <line x1="8" y1="21" x2="16" y2="21"></line>
          <line x1="12" y1="17" x2="12" y2="21"></line>
        </svg>
        Remote Access
      </span>
      <div class="remote-controls">
        <button
          class="remote-lightswitch"
          class:active={$isRemoteActive}
          onclick={handleRemoteToggle}
          disabled={isTogglingRemote}
          aria-label={$isRemoteActive ? 'Stop remote access' : 'Start remote access'}
        >
          <span class="lightswitch-track">
            <span class="lightswitch-thumb"></span>
          </span>
        </button>
        <button
          class="remote-settings-btn"
          onclick={() => remoteAccessViewStore.openSettings()}
          aria-label="Remote access settings"
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
            <circle cx="12" cy="12" r="3"></circle>
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
            ></path>
          </svg>
        </button>
      </div>
    </div>

    {#if remoteError}
      <p class="remote-error">{remoteError}</p>
    {/if}

    {#if $recentProjects.length > 0}
      <section class="recent-section">
        <h2 class="section-title">
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
            <circle cx="12" cy="12" r="10"></circle>
            <polyline points="12 6 12 12 16 14"></polyline>
          </svg>
          Recent Projects
        </h2>
        <div class="recent-list">
          {#each $recentProjects as project, index (project.path)}
            <div class="project-wrapper" style="--index: {index}">
              <RecentProjectItem
                {project}
                onSelect={() => handleProjectSelect(project)}
                onRemove={() => handleProjectRemove(project)}
              />
            </div>
          {/each}
        </div>
      </section>
    {:else}
      <section class="empty-section">
        <div class="empty-icon">
          <svg
            width="40"
            height="40"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
        </div>
        <p class="empty-text">No recent projects</p>
        <p class="empty-hint">Open a directory to start exploring</p>
      </section>
    {/if}
  </div>
</div>

<style>
  .start-screen {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    background: var(--bg-primary);
    overflow: hidden;
  }

  /* ===== Misty Background Layers ===== */
  .bg-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .bg-base {
    background:
      radial-gradient(ellipse 120% 80% at 50% 100%, rgba(125, 211, 252, 0.03) 0%, transparent 50%),
      radial-gradient(ellipse 100% 60% at 50% 0%, rgba(196, 181, 253, 0.02) 0%, transparent 50%),
      var(--bg-primary);
  }

  /* Animated mist layers */
  .bg-mist {
    background: radial-gradient(
      ellipse 100% 30% at 50% 100%,
      rgba(125, 211, 252, 0.04) 0%,
      transparent 70%
    );
  }

  .mist-1 {
    animation: mistDrift 20s ease-in-out infinite;
    transform-origin: center bottom;
  }

  .mist-2 {
    animation: mistDrift 25s ease-in-out infinite reverse;
    opacity: 0.7;
    transform-origin: center bottom;
  }

  .mist-3 {
    animation: mistDrift 30s ease-in-out infinite;
    opacity: 0.5;
    background: radial-gradient(
      ellipse 80% 20% at 30% 80%,
      rgba(196, 181, 253, 0.03) 0%,
      transparent 70%
    );
  }

  @keyframes mistDrift {
    0%,
    100% {
      transform: translateX(-5%) scaleX(1.1);
      opacity: 0.6;
    }
    50% {
      transform: translateX(5%) scaleX(1);
      opacity: 0.4;
    }
  }

  .bg-noise {
    opacity: 0.015;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E");
    background-size: 256px 256px;
  }

  .bg-vignette {
    background: radial-gradient(
      ellipse 70% 60% at center,
      transparent 40%,
      rgba(0, 0, 0, 0.3) 100%
    );
  }

  /* Aurora effect - subtle northern lights */
  .bg-aurora {
    background: linear-gradient(
      120deg,
      transparent 20%,
      rgba(125, 211, 252, 0.015) 35%,
      rgba(196, 181, 253, 0.02) 50%,
      rgba(125, 211, 252, 0.015) 65%,
      transparent 80%
    );
    animation: auroraShift 15s ease-in-out infinite;
    filter: blur(40px);
  }

  @keyframes auroraShift {
    0%,
    100% {
      transform: translateX(-10%) rotate(-2deg);
      opacity: 0.6;
    }
    50% {
      transform: translateX(10%) rotate(2deg);
      opacity: 0.8;
    }
  }

  /* Floating particles */
  .particles {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }

  .particle {
    position: absolute;
    width: 3px;
    height: 3px;
    background: var(--accent-color);
    border-radius: 50%;
    opacity: 0;
    animation: particleFloat 25s ease-in-out infinite;
    animation-delay: calc(var(--i) * 2s);
    left: calc(10% + var(--i) * 7%);
    filter: blur(0.5px);
  }

  .particle:nth-child(odd) {
    background: var(--accent2-color);
    animation-duration: 30s;
  }

  @keyframes particleFloat {
    0% {
      transform: translateY(100vh) scale(0);
      opacity: 0;
    }
    10% {
      opacity: 0.6;
    }
    50% {
      opacity: 0.3;
    }
    90% {
      opacity: 0.5;
    }
    100% {
      transform: translateY(-20vh) scale(1);
      opacity: 0;
    }
  }

  /* ===== Content ===== */
  .content {
    position: relative;
    width: 100%;
    max-width: 480px;
    padding: var(--space-6);
    opacity: 0;
    transform: translateY(20px);
    transition:
      opacity 0.8s ease,
      transform 0.8s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .content.mounted {
    opacity: 1;
    transform: translateY(0);
  }

  /* ===== Header ===== */
  .header {
    text-align: center;
    margin-bottom: var(--space-5);
  }

  .logo-container {
    position: relative;
    display: inline-block;
    margin-bottom: var(--space-3);
  }

  .logo-mist {
    position: absolute;
    inset: -20px;
    background: radial-gradient(circle, rgba(125, 211, 252, 0.08) 0%, transparent 70%);
    animation: logoMist 4s ease-in-out infinite;
  }

  @keyframes logoMist {
    0%,
    100% {
      opacity: 0.2;
      transform: scale(1);
    }
    50% {
      opacity: 0.35;
      transform: scale(1.08);
    }
  }

  .logo {
    position: relative;
    transition: all var(--transition-slow);
  }

  .logo-container:hover .logo {
    opacity: 0.9;
    transform: scale(1.05);
  }

  .logo-container:hover .logo-mist {
    opacity: 0.6;
    transform: scale(1.15);
  }

  .title {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: var(--space-2);
    margin-bottom: var(--space-1);
    cursor: default;
  }

  .title-main {
    font-family: var(--font-display);
    font-size: 28px;
    font-weight: 300;
    letter-spacing: 0.2em;
    color: var(--text-primary);
    transition: color var(--transition-normal);
  }

  .title:hover .title-main {
    color: var(--accent-color);
  }

  .title-kanji {
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 300;
    color: var(--accent-color);
    opacity: 0.5;
    transition: opacity var(--transition-normal);
  }

  .title:hover .title-kanji {
    opacity: 0.8;
  }

  .subtitle {
    font-size: 11px;
    font-weight: 400;
    color: var(--text-muted);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    margin-top: var(--space-2);
    opacity: 0.7;
  }

  /* ===== Open Button ===== */
  .open-button {
    position: relative;
    width: 100%;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition:
      transform var(--transition-normal),
      filter var(--transition-normal);
  }

  .open-button:hover {
    transform: translateY(-2px);
  }

  .open-button:active {
    transform: translateY(0) scale(0.99);
    transition: transform 100ms ease;
    filter: brightness(0.95);
  }

  .button-bg {
    position: absolute;
    inset: 0;
    background: var(--bg-glass);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    transition: all var(--transition-normal);
  }

  .open-button:hover .button-bg {
    background: var(--bg-glass-hover);
    border-color: var(--border-glow);
  }

  .button-content {
    position: relative;
    display: flex;
    align-items: center;
    padding: var(--space-4) var(--space-5);
    gap: var(--space-4);
  }

  .open-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    background: var(--accent-subtle);
    border-radius: var(--radius-md);
    color: var(--accent-color);
    transition: all var(--transition-normal);
  }

  .open-button:hover .open-icon {
    background: var(--accent-muted);
    transform: scale(1.05);
  }

  .open-button:hover .open-icon svg {
    transform: translateY(-1px);
  }

  .open-icon svg {
    transition: transform var(--transition-normal);
  }

  .open-text {
    flex: 1;
    text-align: left;
  }

  .open-title {
    display: block;
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 2px;
    transition: color var(--transition-fast);
  }

  .open-button:hover .open-title {
    color: var(--accent-color);
  }

  .open-desc {
    display: block;
    font-size: 12px;
    color: var(--text-muted);
  }

  .open-shortcut {
    display: flex;
    gap: 4px;
  }

  .open-shortcut kbd {
    padding: 6px 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
    box-shadow: 0 2px 0 var(--bg-primary);
    transition: all var(--transition-fast);
  }

  .open-button:hover .open-shortcut kbd {
    border-color: var(--accent-subtle);
    color: var(--accent-color);
    transform: translateY(-1px);
    box-shadow: 0 3px 0 var(--bg-primary);
  }

  /* ===== Startup Command Row ===== */
  .startup-command-row {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3);
    margin-top: var(--space-3);
    padding: 0 var(--space-1);
  }

  .startup-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 400;
    color: var(--text-muted);
    letter-spacing: 0.04em;
    white-space: nowrap;
  }

  .startup-label svg {
    color: var(--accent-color);
    opacity: 0.4;
  }

  .segment-control {
    display: flex;
    padding: 3px;
    background: rgba(255, 255, 255, 0.03);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-color);
  }

  .segment-option {
    padding: 5px 14px;
    background: transparent;
    border: none;
    border-radius: calc(var(--radius-md) - 3px);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
    white-space: nowrap;
  }

  .segment-option:hover:not(.active) {
    color: var(--text-secondary);
    background: rgba(125, 211, 252, 0.04);
  }

  .segment-option.active {
    background: var(--accent-subtle);
    color: var(--accent-color);
  }

  /* ===== Remote Access Row ===== */
  .remote-controls {
    display: flex;
    align-items: center;
    gap: var(--space-2);
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

  .remote-settings-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: calc(var(--radius-md) - 3px);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .remote-settings-btn:hover {
    color: var(--accent-color);
    background: rgba(125, 211, 252, 0.04);
    border-color: var(--border-color);
  }

  /* ===== Remote Error ===== */
  .remote-error {
    margin-top: var(--space-2);
    padding: 0 var(--space-1);
    font-size: 11px;
    color: var(--accent3-color);
    text-align: right;
    opacity: 0.85;
  }

  /* ===== Recent Section ===== */
  .recent-section {
    margin-top: var(--space-6);
  }

  .section-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    color: var(--text-muted);
    margin-bottom: var(--space-3);
    letter-spacing: 0.1em;
    transition: color var(--transition-fast);
  }

  .section-title:hover {
    color: var(--text-secondary);
  }

  .section-title svg {
    transition:
      transform var(--transition-fast),
      color var(--transition-fast);
  }

  .section-title:hover svg {
    color: var(--accent-color);
    transform: scale(1.1);
  }

  .recent-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    max-height: 280px;
    overflow-y: scroll;
    padding-right: var(--space-2);
    scrollbar-width: thin;
    scrollbar-color: rgba(125, 211, 252, 0.2) transparent;
  }

  .recent-list::-webkit-scrollbar {
    width: 6px;
  }

  .recent-list::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.02);
    border-radius: 3px;
  }

  .recent-list::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.2);
    border-radius: 3px;
  }

  .recent-list::-webkit-scrollbar-thumb:hover {
    background: rgba(125, 211, 252, 0.35);
  }

  .project-wrapper {
    animation: slideUp 0.5s ease backwards;
    animation-delay: calc(var(--index) * 60ms + 300ms);
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* ===== Empty Section ===== */
  .empty-section {
    margin-top: var(--space-7);
    text-align: center;
    padding: var(--space-6);
  }

  .empty-icon {
    color: var(--accent-color);
    opacity: 0.25;
    margin-bottom: var(--space-4);
    transition: all var(--transition-normal);
  }

  .empty-section:hover .empty-icon {
    opacity: 0.35;
    transform: scale(1.05);
  }

  .empty-text {
    color: var(--text-secondary);
    font-size: 15px;
    font-weight: 500;
    margin-bottom: var(--space-1);
  }

  .empty-hint {
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
