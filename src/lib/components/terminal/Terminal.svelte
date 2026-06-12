<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { terminalService } from '@/lib/services/terminalService';
  import { eventService, type UnlistenFn } from '@/lib/services/eventService';
  import type { Terminal as TerminalType } from '@xterm/xterm';
  import type { FitAddon as FitAddonType } from '@xterm/addon-fit';
  import { terminalStore, type PaneColor } from '@/lib/stores/terminalStore';
  import { focusedPaneStore } from '@/lib/stores/focusedPaneStore';
  import { terminalRegistry } from '@/lib/stores/terminalRegistry';
  import { fontSize, startupCommand } from '@/lib/stores/settingsStore';
  import { getStartupCommandString } from '@/lib/services/persistenceService';
  import { peekStore } from '@/lib/stores/peekStore';
  import { notificationService } from '@/lib/services/notificationService';
  import { createFilePathLinkProvider } from '@/lib/services/filePathLinkProvider';
  import TerminalShortcutBar from './TerminalShortcutBar.svelte';
  import TerminalShortcutSettings from './TerminalShortcutSettings.svelte';
  import { shortcutState, isAiProcess } from '@/lib/stores/shortcutStore.svelte';
  import {
    loadShortcuts,
    saveShortcuts,
    loadNumberRowEnabled,
    saveNumberRowEnabled,
  } from '@/lib/services/persistenceService';
  import { getExistingTerminalId, paneExistsInStore } from './terminalPaneHelpers';
  import { createSyncOutputHandler, type SyncOutputHandler } from './terminalSyncOutput';
  import {
    applyPtyRowMargin,
    fitTerminalToContainer as fitTerminal,
    waitForInitialLayout,
  } from './terminalLayout';
  import {
    attachCapturePhaseKeyboard,
    attachKeyEventFilter,
    buildTerminalOptions,
    loadDeferredAddons,
  } from './terminalSetup';
  import {
    PROCESS_POLL_INTERVAL_MS,
    RESIZE_STABILITY_DELAY_MS,
    RESIZE_DEBOUNCE_MS,
  } from './terminalConstants';

  // Lazy-loaded xterm modules (loaded on first terminal creation)
  let xtermLoaded = false;

  interface TerminalOutput {
    id: number;
    data: string;
  }

  interface Props {
    paneId: string;
    cwd?: string | null;
    name?: string;
    color?: PaneColor;
    showControls?: boolean;
    onSplitHorizontal?: () => void;
    onSplitVertical?: () => void;
    onMinimize?: () => void;
    onClose?: () => void;
  }

  let {
    paneId: reactivePaneId,
    cwd = null,
    name = undefined,
    color = undefined,
    showControls = true,
    onSplitHorizontal,
    onSplitVertical,
    onMinimize,
    onClose,
  }: Props = $props();

  // Capture pane identity once at construction. The reactive prop briefly
  // reflects a different value (e.g. the parent split's id like "split-1")
  // during the parent tree restructure that {#key pane.type} in
  // TerminalContainer.svelte triggers on split. Reading it reactively from
  // onDestroy would otherwise make paneExistsInStore() return false and
  // take the "true close" branch — killing the PTY that the reattached
  // Terminal component still needs.
  // svelte-ignore state_referenced_locally
  const paneId = reactivePaneId;

  let terminalWrapper: HTMLDivElement;
  let terminalPadding: HTMLDivElement;
  let terminalContainer: HTMLDivElement;
  let terminal: TerminalType | null = null;
  let fitAddon: FitAddonType | null = null;
  let terminalId: number | null = null;
  let unlisten: UnlistenFn | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let isFocused = $state(false);
  let processName = $state('');
  let showShortcutSettings = $state(false);
  let shortcutFocusSection = $state<'reply' | 'command' | null>(null);
  let numberRowEnabled = $state(false);
  let collapsed = $state(false);
  const unsubscribeCollapsed = terminalStore.subscribe(() => {
    collapsed = terminalStore.isCollapsed(paneId);
  });
  const isAiRunning = $derived(isAiProcess(processName));

  // Drop (not buffer) output during a resize that happens after initial
  // setup: buffered content is sized for the OLD terminal, and replaying
  // it after the resize corrupts Ink layouts. The Ink app will redraw on
  // its SIGWINCH anyway. terminalSyncOutput.ts owns the actual drop.
  let syncHandler: SyncOutputHandler | null = null;
  let resizeStabilityTimeout: ReturnType<typeof setTimeout> | null = null;

  // Track last sent PTY size to prevent duplicate resize calls
  let lastSentPtySize: { cols: number; rows: number } | null = null;

  // Focus terminal once it is ready (single-terminal model: always active)
  $effect(() => {
    if (terminal) {
      requestAnimationFrame(() => {
        terminal?.focus();
        requestAnimationFrame(() => {
          fitTerminalToContainer();
        });
      });
    }
  });

  function attachFocusTracking(termInstance: TerminalType) {
    termInstance.textarea?.addEventListener('focus', () => {
      isFocused = true;
      focusedPaneStore.set(paneId);
    });
    termInstance.textarea?.addEventListener('blur', () => {
      isFocused = false;
    });
  }

  /**
   * Reattach an existing xterm instance from the registry to our new
   * container. Triggered when a split rebuilds the parent tree and the
   * Terminal component remounts: we move xterm's managed DOM node into
   * the freshly mounted container rather than creating a new terminal.
   */
  function reattachFromRegistry(existingInstance: ReturnType<typeof terminalRegistry.get>) {
    if (!existingInstance) return;
    terminal = existingInstance.terminal;
    fitAddon = existingInstance.fitAddon;
    terminalId = existingInstance.terminalId;
    unlisten = existingInstance.unlisten;

    if (terminal.element) {
      // xterm.js manages its own DOM, so we manually move it. We don't
      // check parent — when the previous container is removed from the
      // DOM the element is orphaned but still re-appendable.
      // eslint-disable-next-line svelte/no-dom-manipulating
      terminalContainer.appendChild(terminal.element);
      // WebGL context may need a refresh after the DOM move.
      terminal.refresh(0, terminal.rows - 1);
    } else {
      terminal.open(terminalContainer);
    }

    document.fonts.ready.then(() => {
      setTimeout(() => {
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            fitTerminalToContainer();
            terminal?.refresh(0, terminal.rows - 1);
            if (terminal && (terminal.cols < 40 || terminal.rows < 10)) {
              setTimeout(() => {
                fitTerminalToContainer();
                terminal?.refresh(0, terminal.rows - 1);
                terminal?.focus();
              }, 100);
            } else {
              terminal?.focus();
            }
          });
        });
      }, 100);
    });

    attachFocusTracking(terminal);
  }

  async function ensureXtermLoaded(): Promise<{
    TerminalCtor: typeof import('@xterm/xterm').Terminal;
    FitAddonCtor: typeof import('@xterm/addon-fit').FitAddon;
  }> {
    if (!xtermLoaded) {
      await import('@xterm/xterm/css/xterm.css');
      xtermLoaded = true;
    }
    // Only Terminal core + FitAddon are needed for the first render.
    // WebLinks/Canvas are deferred (loadDeferredAddons) until after the
    // terminal is on screen so they stay off the critical path.
    const [{ Terminal: TerminalCtor }, { FitAddon: FitAddonCtor }] = await Promise.all([
      import('@xterm/xterm'),
      import('@xterm/addon-fit'),
    ]);
    return { TerminalCtor, FitAddonCtor };
  }

  async function initTerminal() {
    const existingInstance = terminalRegistry.get(paneId);
    if (existingInstance) {
      reattachFromRegistry(existingInstance);
      return;
    }

    const existingTerminalId = getExistingTerminalId(paneId);
    const { TerminalCtor, FitAddonCtor } = await ensureXtermLoaded();

    terminal = new TerminalCtor(buildTerminalOptions(get(fontSize)));
    fitAddon = new FitAddonCtor();
    terminal.loadAddon(fitAddon);

    void loadDeferredAddons(terminal);

    terminal.registerLinkProvider(
      createFilePathLinkProvider(terminal, (path, line, column) => {
        peekStore.open(path, line, column);
      })
    );

    attachKeyEventFilter(terminal);

    // Guard against the component unmounting before the async xterm
    // imports resolve (rapid mount/unmount during tests, or a fast pane
    // teardown). Without this xterm throws "Terminal requires a parent
    // element" on a detached node.
    if (!terminalContainer?.isConnected) {
      return;
    }
    terminal.open(terminalContainer);

    try {
      if (existingTerminalId !== null) {
        terminalId = existingTerminalId;
        await waitForInitialLayout(terminal, fitAddon, terminalContainer);
      } else {
        await waitForInitialLayout(terminal, fitAddon, terminalContainer);
        // Cols is already capped by waitForInitialLayout(); we apply the
        // PTY row margin so Ink apps don't flicker at full height.
        const cols = terminal.cols;
        const rows = applyPtyRowMargin(terminal.rows);
        terminalId = await terminalService.createTerminal(cwd, cols, rows);
        terminalStore.setTerminalId(paneId, terminalId);
      }

      terminal?.focus();

      syncHandler = createSyncOutputHandler(terminal);

      unlisten = await eventService.listen<TerminalOutput>('terminal-output', (event) => {
        if (event.payload.id !== terminalId || !terminal || !syncHandler) return;
        let data = event.payload.data;

        // Notifications (OSC 9, OSC 777) are stripped from the output
        // and fired asynchronously so they don't block the write path.
        const { output: cleanedData, notifications } = notificationService.parseNotifications(data);
        data = cleanedData;
        for (const notification of notifications) {
          notificationService.notify(notification);
        }

        syncHandler.process(data);
      });

      if (terminal && fitAddon && terminalId !== null && unlisten) {
        terminalRegistry.register(paneId, { terminal, fitAddon, terminalId, unlisten });
      }

      // Execute startup command for freshly created root panes only —
      // reattached panes already ran it on their original mount, and
      // split children inherit their parent's shell.
      if (existingTerminalId === null) {
        const state = terminalStore.getState();
        const isRootTerminalPane =
          state.rootPane?.type === 'terminal' && state.rootPane.id === paneId;
        if (isRootTerminalPane) {
          // get(startupCommand) is a one-shot read; not a reactivity bypass.
          const commandStr = getStartupCommandString(get(startupCommand));
          if (commandStr) {
            setTimeout(() => {
              if (terminalId !== null) {
                terminalService.writeTerminal(terminalId, commandStr + '\n');
              }
            }, 300);
          }
        }
      }

      terminal.onData((data) => {
        if (terminalId !== null) {
          terminalService.writeTerminal(terminalId, data);
        }
      });

      terminal.onResize(({ cols, rows }) => {
        if (terminalId === null) return;
        const ptyRows = applyPtyRowMargin(rows);
        if (lastSentPtySize && lastSentPtySize.cols === cols && lastSentPtySize.rows === ptyRows) {
          return;
        }
        lastSentPtySize = { cols, rows: ptyRows };
        terminalService.resizeTerminal(terminalId, cols, ptyRows);
      });

      // PTY was created with the correct initial size, but dispatching
      // terminal-resize twice mirrors the path split panes take so the
      // visual state and the PTY agree even if layout shifts late.
      setTimeout(() => window.dispatchEvent(new Event('terminal-resize')), 100);
      setTimeout(() => window.dispatchEvent(new Event('terminal-resize')), 250);

      // Past this point, mid-stream resize drops output to prevent Ink
      // glitches. Until then, the shell's first prompt is valid even
      // if a resize fires during initial layout.
      setTimeout(() => syncHandler?.markInitialSetupComplete(), 400);

      attachFocusTracking(terminal);
      attachCapturePhaseKeyboard(terminal, (data) => {
        if (terminalId !== null) {
          terminalService.writeTerminal(terminalId, data);
        }
      });
    } catch (error) {
      console.error('Failed to create terminal:', error);
      terminal.write(`\r\n\x1b[31mError: Failed to create terminal: ${error}\x1b[0m\r\n`);
    }
  }

  let resizeTimeout: ReturnType<typeof setTimeout> | null = null;

  function fitTerminalToContainer() {
    if (!fitAddon || !terminal || !terminalContainer) return;
    fitTerminal(terminal, fitAddon, terminalContainer);
  }

  function scheduleResizeEnd() {
    if (resizeStabilityTimeout) {
      clearTimeout(resizeStabilityTimeout);
    }
    resizeStabilityTimeout = setTimeout(() => {
      requestAnimationFrame(() => syncHandler?.setResizing(false));
    }, RESIZE_STABILITY_DELAY_MS);
  }

  function handleResize() {
    if (!terminal) return;

    syncHandler?.setResizing(true);

    if (resizeTimeout) {
      clearTimeout(resizeTimeout);
    }
    // Debounce ~1 frame to coalesce rapid window-resize events; the RAF
    // inside ensures layout has committed before we re-fit.
    resizeTimeout = setTimeout(() => {
      requestAnimationFrame(() => {
        fitTerminalToContainer();
        scheduleResizeEnd();
      });
    }, RESIZE_DEBOUNCE_MS);
  }

  // Poll foreground process name to drive the AI shortcut bar
  let processPollInterval: ReturnType<typeof setInterval> | null = null;

  async function updateProcessInfo() {
    if (terminalId === null) return;
    try {
      const info = await terminalService.getProcessInfo(terminalId);
      processName = info.name;
    } catch {
      // Terminal may have been closed
    }
  }

  function handleShortcutSend(text: string, withEnter: boolean) {
    if (terminalId === null) return;
    terminalService.writeTerminal(terminalId, withEnter ? text + '\r' : text);
    terminal?.focus();
  }

  async function handleShortcutAdd(
    label: string,
    text: string,
    type: 'reply' | 'command' = 'reply'
  ) {
    shortcutState.addShortcut(label, text, type);
    await saveShortcuts(shortcutState.customShortcuts);
  }

  async function handleShortcutUpdate(id: string, label: string, text: string) {
    shortcutState.updateShortcut(id, label, text);
    await saveShortcuts(shortcutState.customShortcuts);
  }

  async function handleShortcutRemove(id: string) {
    shortcutState.removeShortcut(id);
    await saveShortcuts(shortcutState.customShortcuts);
  }

  function handleShortcutAddClick(type: 'reply' | 'command') {
    shortcutFocusSection = type;
    showShortcutSettings = true;
  }

  async function handleNumberRowToggle(enabled: boolean) {
    numberRowEnabled = enabled;
    shortcutState.numberRowEnabled = enabled;
    await saveNumberRowEnabled(enabled);
  }

  /**
   * Load per-pane shortcut/settings state. Runs on every mount so that
   * panes reattached after a split get their state populated too (the
   * registry-reuse path of initTerminal short-circuits before reaching here).
   */
  async function loadPaneState() {
    const customShortcuts = await loadShortcuts();
    shortcutState.setCustomShortcuts(customShortcuts);
    numberRowEnabled = await loadNumberRowEnabled();
  }

  onMount(() => {
    initTerminal();
    loadPaneState();

    // Initialize notification service for OSC 9/777 notifications
    notificationService.init();

    // Start process info polling after terminal initializes
    setTimeout(() => {
      updateProcessInfo();
      processPollInterval = setInterval(updateProcessInfo, PROCESS_POLL_INTERVAL_MS);
    }, 1500);

    // Subscribe to font size changes and update terminal
    const unsubscribeFontSize = fontSize.subscribe((size) => {
      if (terminal) {
        terminal.options.fontSize = size;
        // Fit terminal to container after font size change
        requestAnimationFrame(() => {
          fitTerminalToContainer();
        });
      }
    });

    // Use ResizeObserver to detect container size changes
    resizeObserver = new ResizeObserver((entries) => {
      // Only resize if the size actually changed meaningfully
      const entry = entries[0];
      if (entry && entry.contentRect.width > 0 && entry.contentRect.height > 0) {
        handleResize();
      }
    });
    // Observe the wrapper element which is the direct child of the split pane
    resizeObserver.observe(terminalWrapper);
    // Also observe the padding element to detect size changes from shortcut bar visibility
    resizeObserver.observe(terminalPadding);

    // Also listen for window resize as a fallback
    window.addEventListener('resize', handleResize);

    // Listen for custom terminal-resize event (dispatched when pane sizes change)
    const handleTerminalResize = () => {
      syncHandler?.setResizing(true);
      // Force immediate resize without debounce (pane size changes are
      // discrete events, not continuous like window resize).
      requestAnimationFrame(() => {
        fitTerminalToContainer();
        scheduleResizeEnd();
      });
    };
    window.addEventListener('terminal-resize', handleTerminalResize);

    return () => {
      unsubscribeFontSize();
      window.removeEventListener('terminal-resize', handleTerminalResize);
    };
  });

  onDestroy(() => {
    unsubscribeCollapsed();

    if (processPollInterval) {
      clearInterval(processPollInterval);
    }

    if (resizeTimeout) {
      clearTimeout(resizeTimeout);
    }

    if (resizeStabilityTimeout) {
      clearTimeout(resizeStabilityTimeout);
    }

    window.removeEventListener('resize', handleResize);

    if (resizeObserver) {
      resizeObserver.disconnect();
    }

    // If the pane still exists in the store this is just a remount
    // triggered by a split — preserve the terminal for the reattach path.
    const paneStillExists = paneExistsInStore(paneId);

    if (paneStillExists) {
      // Pane still exists, terminal will be reattached
      // Don't dispose anything, keep the instance in the registry
      console.log(`[Terminal] Preserving terminal for paneId=${paneId}`);
      return;
    }

    // Pane is being truly closed - clean up everything
    console.log(`[Terminal] Closing terminal for paneId=${paneId}, terminalId=${terminalId}`);
    terminalRegistry.remove(paneId);

    if (unlisten) {
      unlisten();
    }

    if (terminalId !== null) {
      terminalService.closeTerminal(terminalId);
    }

    if (terminal) {
      try {
        terminal.dispose();
      } catch {
        // WebGL/Canvas addon may throw during dispose if DOM is already detached
      }
    }
  });
</script>

<div
  class="terminal-wrapper"
  data-testid="terminal"
  class:focused={isFocused}
  bind:this={terminalWrapper}
>
  {#if showControls}
    <div class="terminal-controls">
      <button
        class="control-btn"
        onclick={onSplitVertical}
        title="Split Vertically"
        aria-label="Split Vertically"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <rect x="3" y="3" width="18" height="18" rx="2" />
          <line x1="12" y1="3" x2="12" y2="21" />
        </svg>
      </button>
      <button
        class="control-btn"
        onclick={onSplitHorizontal}
        title="Split Horizontally"
        aria-label="Split Horizontally"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <rect x="3" y="3" width="18" height="18" rx="2" />
          <line x1="3" y1="12" x2="21" y2="12" />
        </svg>
      </button>
      <span class="trailing-spacer"></span>
      {#if name || color}
        <span
          class="pane-label"
          style:--pane-color={color ? `var(--pane-color-${color})` : 'transparent'}
        >
          {#if color}<span class="pane-dot" aria-hidden="true"></span>{/if}
          {#if name}<span class="pane-name">{name}</span>{/if}
        </span>
      {/if}
      {#if onMinimize}
        <button
          class="control-btn"
          onclick={onMinimize}
          title="Minimize to dock"
          aria-label="Minimize to dock"
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
            <path d="M12 4v10" />
            <path d="M8 11l4 4 4-4" />
            <line x1="6" y1="20" x2="18" y2="20" />
          </svg>
        </button>
      {/if}
      {#if onClose}
        <button
          class="control-btn close-btn"
          onclick={onClose}
          title="Close Terminal"
          aria-label="Close Terminal"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      {/if}
    </div>
  {/if}
  <div class="terminal-padding" bind:this={terminalPadding}>
    <div class="terminal-container" bind:this={terminalContainer}></div>
  </div>
  <TerminalShortcutBar
    visible={isAiRunning}
    shortcuts={shortcutState.allShortcuts}
    showNumberRow={numberRowEnabled}
    {collapsed}
    onSend={handleShortcutSend}
    onSettingsClick={() => {
      shortcutFocusSection = null;
      showShortcutSettings = true;
    }}
    onAddClick={handleShortcutAddClick}
    onToggleCollapse={() => terminalStore.toggleCollapsed(paneId)}
  />
  <TerminalShortcutSettings
    open={showShortcutSettings}
    shortcuts={shortcutState.allShortcuts}
    focusSection={shortcutFocusSection}
    {numberRowEnabled}
    onClose={() => {
      showShortcutSettings = false;
      shortcutFocusSection = null;
    }}
    onAdd={handleShortcutAdd}
    onUpdate={handleShortcutUpdate}
    onRemove={handleShortcutRemove}
    onNumberRowToggle={handleNumberRowToggle}
  />
  <div class="terminal-glow"></div>
  <div class="focus-indicator"></div>
  <div class="scanlines"></div>
  <div class="crt-curve"></div>
</div>

<style>
  .terminal-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: linear-gradient(180deg, #0a0c10 0%, #0c0e14 100%);
    overflow: hidden;
    animation: terminalFadeIn 0.4s ease-out;
  }

  @keyframes terminalFadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  /* Terminal controls */
  .terminal-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    height: var(--tabbar-height, 44px);
    padding: 0 8px;
    background: rgba(10, 12, 16, 0.8);
    border-bottom: 1px solid rgba(125, 211, 252, 0.1);
    z-index: 10;
    flex-shrink: 0;
  }

  .control-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .control-btn:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
  }

  .control-btn.close-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  /* Flex spacer that pushes the trailing cluster (pane-label, close-btn) to
     the right. Using a single spacer keeps the trailing group visually tight
     regardless of which of those elements are present. */
  .trailing-spacer {
    flex: 1 1 auto;
  }

  .pane-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    font-family: 'IBM Plex Mono', 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-secondary);
    letter-spacing: 0.04em;
  }

  .pane-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--pane-color);
    box-shadow: 0 0 6px 0.5px color-mix(in srgb, var(--pane-color) 60%, transparent);
    flex-shrink: 0;
  }

  .pane-name {
    white-space: nowrap;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Ambient corner glow */
  .terminal-wrapper::before {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    width: 200px;
    height: 200px;
    background: radial-gradient(
      circle at bottom left,
      rgba(125, 211, 252, 0.03) 0%,
      transparent 70%
    );
    pointer-events: none;
    z-index: 0;
  }

  .terminal-wrapper::after {
    content: '';
    position: absolute;
    top: 0;
    right: 0;
    width: 200px;
    height: 200px;
    background: radial-gradient(circle at top right, rgba(196, 181, 253, 0.02) 0%, transparent 70%);
    pointer-events: none;
    z-index: 0;
  }

  .terminal-padding {
    position: relative;
    flex: 1;
    min-height: 0;
    padding: var(--terminal-padding-y) var(--terminal-padding-x);
    box-sizing: border-box;
    overflow: hidden;
  }

  .terminal-container {
    width: 100%;
    height: 100%;
  }

  /* Atmospheric glow effect at the top */
  .terminal-glow {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(125, 211, 252, 0.3) 50%,
      transparent 100%
    );
    opacity: 0.4;
    pointer-events: none;
    transition: all var(--transition-normal);
  }

  .terminal-wrapper.focused .terminal-glow {
    opacity: 0.8;
    height: 2px;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(125, 211, 252, 0.5) 30%,
      rgba(196, 181, 253, 0.5) 70%,
      transparent 100%
    );
  }

  /* Focus indicator - gradient left border */
  .focus-indicator {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 2px;
    background: linear-gradient(180deg, var(--gradient-start), var(--gradient-end));
    opacity: 0;
    transition: opacity var(--transition-normal);
    pointer-events: none;
  }

  .terminal-wrapper.focused .focus-indicator {
    opacity: 1;
  }

  /* xterm overrides for the mist theme */
  .terminal-container :global(.xterm) {
    width: 100%;
    height: 100%;
  }

  .terminal-container :global(.xterm-screen) {
    width: 100% !important;
  }

  .terminal-container :global(.xterm-rows) {
    width: 100% !important;
  }

  .terminal-container :global(.xterm-rows > div) {
    width: 100% !important;
  }

  .terminal-container :global(.xterm-viewport) {
    overflow-y: auto !important;
    scrollbar-width: thin;
    scrollbar-color: rgba(125, 211, 252, 0.15) transparent;
  }

  .terminal-container :global(.xterm-viewport::-webkit-scrollbar) {
    width: 8px;
  }

  .terminal-container :global(.xterm-viewport::-webkit-scrollbar-track) {
    background: transparent;
  }

  .terminal-container :global(.xterm-viewport::-webkit-scrollbar-thumb) {
    background: rgba(125, 211, 252, 0.12);
    border-radius: 4px;
  }

  .terminal-container :global(.xterm-viewport::-webkit-scrollbar-thumb:hover) {
    background: rgba(125, 211, 252, 0.2);
  }

  /* Selection styling */
  .terminal-container :global(.xterm-selection div) {
    background: rgba(125, 211, 252, 0.25) !important;
    border-radius: 2px;
  }

  /* Link styling */
  .terminal-container :global(.xterm-underline-1) {
    text-decoration: underline;
    text-decoration-color: var(--accent-color);
    text-underline-offset: 2px;
  }

  /* Smooth text rendering */
  .terminal-container :global(.xterm-screen) {
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  /* Scanline overlay effect */
  .scanlines {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: repeating-linear-gradient(
      0deg,
      transparent,
      transparent 2px,
      rgba(0, 0, 0, 0.02) 2px,
      rgba(0, 0, 0, 0.02) 4px
    );
    z-index: 5;
    opacity: 0.4;
    transition: opacity var(--transition-normal);
  }

  .terminal-wrapper.focused .scanlines {
    opacity: 0.3;
  }

  /* CRT screen curvature effect */
  .crt-curve {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 6;
    background: radial-gradient(
      ellipse 120% 100% at 50% 50%,
      transparent 70%,
      rgba(0, 0, 0, 0.15) 100%
    );
    opacity: 0.6;
  }

  .terminal-wrapper.focused .crt-curve {
    opacity: 0.4;
  }
</style>
