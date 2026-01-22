<script lang="ts">
  import { tabStore, activeTab, getAllPaneIds } from '@/lib/stores/tabStore';
  import { currentProjectPath } from '@/lib/stores/projectStore';
  import { TerminalContainer } from '@/lib/components/terminal';
  import { Editor } from '@/lib/components/editor';
  import TabBar from './TabBar.svelte';

  function handleEditorModified(tabId: string, modified: boolean) {
    tabStore.setModified(tabId, modified);
  }

  // Counter to force {#key} block to re-render when tabs change
  // This is needed because Svelte's {#key} doesn't properly re-render
  // when the key returns to a previous value (A -> B -> A)
  let tabChangeCounter = $state(0);
  let prevActiveTabId = $state<string | null>(null);
  let prevTabCount = $state(0);

  $effect(() => {
    const currentActiveTabId = $tabStore.activeTabId;
    const currentTabCount = $tabStore.tabs.length;

    if (prevActiveTabId !== currentActiveTabId || prevTabCount !== currentTabCount) {
      tabChangeCounter++;
      prevActiveTabId = currentActiveTabId;
      prevTabCount = currentTabCount;
    }
  });
</script>

<main class="main-content">
  <TabBar tabs={$tabStore.tabs} activeTabId={$tabStore.activeTabId} />
  <div class="content-area">
    {#key tabChangeCounter}
      {#if $activeTab}
        {#if $activeTab.type === 'terminal'}
          <TerminalContainer
            tabId={$activeTab.id}
            pane={$activeTab.rootPane}
            cwd={$currentProjectPath}
            isOnlyPane={getAllPaneIds($activeTab.rootPane).length === 1}
          />
        {:else if $activeTab.type === 'editor'}
          <Editor
            filePath={$activeTab.filePath}
            onModifiedChange={(modified) => handleEditorModified($activeTab.id, modified)}
          />
        {/if}
      {:else}
        <div class="no-tabs">
          <div class="bg-layer bg-gradient"></div>
          <div class="bg-layer bg-noise"></div>
          <div class="bg-layer bg-grid"></div>
          <div class="bg-layer bg-aurora"></div>

          <!-- Floating particles -->
          <div class="particles">
            {#each Array(8) as _, i (i)}
              <div class="particle" style="--i: {i}"></div>
            {/each}
          </div>

          <div class="empty-state">
            <div class="empty-icon-container">
              <div class="icon-glow"></div>
              <div class="empty-icon">
                <svg
                  width="56"
                  height="56"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <polyline points="4 17 10 11 4 5"></polyline>
                  <line x1="12" y1="19" x2="20" y2="19"></line>
                </svg>
              </div>
            </div>
            <h2 class="empty-title">No tabs open</h2>
            <p class="empty-description">Open a terminal or select a file from the explorer</p>
            <button class="open-terminal-btn" onclick={() => tabStore.addTerminalTab()}>
              <span class="btn-icon">
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
                  <line x1="12" y1="5" x2="12" y2="19"></line>
                  <line x1="5" y1="12" x2="19" y2="12"></line>
                </svg>
              </span>
              <span>New Terminal</span>
            </button>
            <p class="shortcut-hint">
              <kbd>⌘</kbd> + <kbd>`</kbd>
            </p>
          </div>
        </div>
      {/if}
    {/key}
  </div>
</main>

<style>
  .main-content {
    flex: 1;
    height: 100%;
    background: var(--bg-primary);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .content-area {
    flex: 1;
    overflow: hidden;
  }

  .no-tabs {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    overflow: hidden;
  }

  .bg-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .bg-gradient {
    background:
      radial-gradient(ellipse 60% 50% at 50% 30%, rgba(125, 211, 252, 0.04) 0%, transparent 60%),
      radial-gradient(ellipse 80% 60% at 80% 80%, rgba(196, 181, 253, 0.03) 0%, transparent 60%),
      linear-gradient(180deg, var(--bg-primary) 0%, var(--bg-secondary) 100%);
  }

  .bg-noise {
    opacity: 0.02;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E");
    background-size: 256px 256px;
  }

  .empty-state {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-4);
    text-align: center;
    animation: fadeIn 0.5s ease;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(16px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .empty-icon-container {
    position: relative;
    margin-bottom: var(--space-2);
  }

  .icon-glow {
    position: absolute;
    inset: -24px;
    background: radial-gradient(circle, rgba(125, 211, 252, 0.1) 0%, transparent 70%);
    opacity: 0.25;
    transition: opacity var(--transition-normal);
  }

  .empty-icon-container:hover .icon-glow {
    opacity: 0.4;
  }

  .empty-icon {
    position: relative;
    color: var(--accent-color);
    opacity: 0.4;
    transition: all var(--transition-normal);
  }

  .empty-icon-container:hover .empty-icon {
    opacity: 0.6;
    transform: scale(1.05);
  }

  .empty-title {
    font-size: 16px;
    font-weight: 500;
    margin: 0;
    letter-spacing: -0.01em;
    color: var(--text-secondary);
  }

  .empty-description {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0;
    animation: descFade 0.5s ease 0.2s backwards;
  }

  @keyframes descFade {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .open-terminal-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
    background: var(--accent-color);
    border: none;
    border-radius: var(--radius-md);
    color: var(--bg-primary);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-normal);
    margin-top: var(--space-2);
  }

  .open-terminal-btn:hover {
    transform: translateY(-2px);
    background: var(--accent-secondary);
  }

  .open-terminal-btn:active {
    transform: translateY(0) scale(0.98);
    transition: transform 100ms ease;
  }

  .btn-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform var(--transition-fast);
  }

  .open-terminal-btn:hover .btn-icon {
    transform: rotate(90deg);
  }

  .shortcut-hint {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
    margin-top: var(--space-2);
    animation: hintFade 0.5s ease 0.4s backwards;
  }

  @keyframes hintFade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  kbd {
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
    box-shadow: 0 2px 0 var(--bg-primary);
    transition: all var(--transition-fast);
  }

  .shortcut-hint:hover kbd {
    border-color: var(--accent-subtle);
    color: var(--accent-color);
    transform: translateY(-1px);
    box-shadow: 0 3px 0 var(--bg-primary);
  }

  /* Button ripple effect */
  .open-terminal-btn {
    position: relative;
    overflow: hidden;
  }

  .open-terminal-btn::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.15), transparent);
    transform: translateX(-100%);
    transition: transform 0.5s ease;
  }

  .open-terminal-btn:hover::before {
    transform: translateX(100%);
  }

  /* Grid background */
  .bg-grid {
    background-image:
      linear-gradient(rgba(125, 211, 252, 0.015) 1px, transparent 1px),
      linear-gradient(90deg, rgba(125, 211, 252, 0.015) 1px, transparent 1px);
    background-size: 80px 80px;
  }

  .bg-aurora {
    background: linear-gradient(
      120deg,
      transparent 20%,
      rgba(125, 211, 252, 0.02) 35%,
      rgba(196, 181, 253, 0.025) 50%,
      rgba(125, 211, 252, 0.02) 65%,
      transparent 80%
    );
    animation: auroraShift 15s ease-in-out infinite;
    filter: blur(60px);
  }

  @keyframes auroraShift {
    0%,
    100% {
      transform: translateX(-10%) rotate(-2deg);
      opacity: 0.6;
    }
    50% {
      transform: translateX(10%) rotate(2deg);
      opacity: 1;
    }
  }

  .particles {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .particle {
    position: absolute;
    width: 3px;
    height: 3px;
    background: var(--accent-color);
    border-radius: 50%;
    opacity: 0.3;
    animation: particleFloat 12s ease-in-out infinite;
    animation-delay: calc(var(--i) * -1.5s);
    left: calc(15% + var(--i) * 10%);
    top: calc(20% + (var(--i) * 7%));
    filter: blur(1px);
  }

  .particle:nth-child(odd) {
    background: var(--accent2-color);
    animation-duration: 15s;
  }

  @keyframes particleFloat {
    0%,
    100% {
      transform: translate(0, 0) scale(1);
      opacity: 0.2;
    }
    25% {
      transform: translate(20px, -30px) scale(1.2);
      opacity: 0.5;
    }
    50% {
      transform: translate(-10px, -20px) scale(0.8);
      opacity: 0.3;
    }
    75% {
      transform: translate(15px, 10px) scale(1.1);
      opacity: 0.4;
    }
  }

  /* Content area subtle border */
  .content-area {
    position: relative;
  }

  .content-area::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: linear-gradient(90deg, transparent, rgba(125, 211, 252, 0.05), transparent);
    pointer-events: none;
    z-index: 1;
  }
</style>
