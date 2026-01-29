<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { terminalService } from '@/lib/services/terminalService';
  import { eventService, type UnlistenFn } from '@/lib/services/eventService';
  import type { Terminal as TerminalType } from '@xterm/xterm';
  import type { FitAddon as FitAddonType } from '@xterm/addon-fit';
  import { tabStore, getAllPaneIds, type TerminalTab } from '@/lib/stores/tabStore';
  import { terminalRegistry } from '@/lib/stores/terminalRegistry';
  import { fontSize } from '@/lib/stores/settingsStore';
  import { peekStore } from '@/lib/stores/peekStore';
  import { openerService } from '@/lib/services/openerService';
  import { notificationService } from '@/lib/services/notificationService';
  import { createFilePathLinkProvider } from '@/lib/services/filePathLinkProvider';

  // Lazy-loaded xterm modules (loaded on first terminal creation)
  let xtermLoaded = false;

  interface TerminalOutput {
    id: number;
    data: string;
  }

  interface Props {
    tabId: string;
    paneId: string;
    cwd?: string | null;
    showControls?: boolean;
    onSplitHorizontal?: () => void;
    onSplitVertical?: () => void;
    onClose?: () => void;
  }

  let {
    tabId,
    paneId,
    cwd = null,
    showControls = true,
    onSplitHorizontal,
    onSplitVertical,
    onClose,
  }: Props = $props();

  let terminalWrapper: HTMLDivElement;
  let terminalPadding: HTMLDivElement;
  let terminalContainer: HTMLDivElement;
  let terminal: TerminalType | null = null;
  let fitAddon: FitAddonType | null = null;
  let terminalId: number | null = null;
  let unlisten: UnlistenFn | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let isFocused = $state(false);

  // Maximum terminal width to prevent rendering issues with Ink-based apps
  // Wide terminals (140+ cols) can cause spinner glitches due to cursor movement calculations
  const MAX_TERMINAL_COLS = 120;

  // Reserve 1 row for PTY to prevent Ink full-height flickering issue
  // See: https://github.com/vadimdemedes/ink/issues/450
  // When Ink renders at exactly terminal height, unintended scrolling occurs
  const PTY_ROW_MARGIN = 1;

  // Resize stability: drop output during resize to prevent partial frame rendering
  // This acts as an additional sync layer on top of Mode 2026
  // NOTE: We DROP (not buffer) output during resize because:
  // - Buffered content is rendered for the OLD terminal size
  // - Writing old-size content after resize causes cursor position mismatches
  // - This manifests as character corruption and unnatural line breaks in Ink apps
  // - The Ink app will receive SIGWINCH and redraw with the new size anyway
  let isResizing = false;
  let resizeStabilityTimeout: ReturnType<typeof setTimeout> | null = null;
  const RESIZE_STABILITY_DELAY = 50; // ms to wait after resize before resuming output

  // Track last sent PTY size to prevent duplicate resize calls
  // This is needed because fitTerminalToContainer() may trigger onResize twice:
  // once from fitAddon.fit() and once from terminal.resize() for MAX_TERMINAL_COLS capping
  let lastSentPtySize: { cols: number; rows: number } | null = null;

  // Watch for tab activation to focus terminal
  const isActiveTab = $derived($tabStore.activeTabId === tabId);

  $effect(() => {
    if (isActiveTab && terminal) {
      // Small delay to ensure the tab is fully rendered
      requestAnimationFrame(() => {
        terminal?.focus();
        // Force fit when tab becomes active
        // This handles the case when switching tabs or closing other tabs
        // Use double rAF to ensure layout is fully settled
        requestAnimationFrame(() => {
          fitTerminalToContainer();
        });
      });
    }
  });

  // Watch for tab count changes and trigger resize
  // This ensures proper fit when tabs are added or removed
  const tabCount = $derived($tabStore.tabs.length);
  let prevTabCount = $state(0);

  $effect(() => {
    if (prevTabCount !== 0 && prevTabCount !== tabCount && terminal) {
      // Tab count changed - trigger resize after layout settles
      setTimeout(() => {
        requestAnimationFrame(() => {
          fitTerminalToContainer();
        });
      }, 50);
    }
    prevTabCount = tabCount;
  });

  // KIRI Mist Theme - soft atmospheric terminal colors
  const mistTheme = {
    background: '#0a0c10',
    foreground: '#c8d3e0',
    cursor: '#7dd3fc',
    cursorAccent: '#0a0c10',
    selectionBackground: 'rgba(125, 211, 252, 0.2)',
    selectionForeground: '#f0f4f8',
    // ANSI colors - soft, muted palette
    black: '#0e1218',
    red: '#f87171',
    green: '#4ade80',
    yellow: '#fbbf24',
    blue: '#7dd3fc',
    magenta: '#c4b5fd',
    cyan: '#67e8f9',
    white: '#c8d3e0',
    brightBlack: '#5c6b7a',
    brightRed: '#fca5a5',
    brightGreen: '#86efac',
    brightYellow: '#fcd34d',
    brightBlue: '#93c5fd',
    brightMagenta: '#d8b4fe',
    brightCyan: '#a5f3fc',
    brightWhite: '#f0f4f8',
  };

  /**
   * Check if this pane still exists in the tab store
   */
  function paneExistsInStore(): boolean {
    const state = get(tabStore);
    const tab = state.tabs.find((t) => t.id === tabId);
    if (!tab || tab.type !== 'terminal') return false;
    const terminalTab = tab as TerminalTab;
    const allPaneIds = getAllPaneIds(terminalTab.rootPane);
    return allPaneIds.includes(paneId);
  }

  /**
   * Get existing terminal ID from the pane in store
   */
  function getExistingTerminalId(): number | null {
    const state = get(tabStore);
    const tab = state.tabs.find((t) => t.id === tabId);
    if (!tab || tab.type !== 'terminal') return null;

    const terminalTab = tab as TerminalTab;
    const findTerminalId = (pane: typeof terminalTab.rootPane): number | null => {
      if (pane.type === 'terminal') {
        if (pane.id === paneId) return pane.terminalId;
        return null;
      }
      for (const child of pane.children) {
        const result = findTerminalId(child);
        if (result !== null) return result;
      }
      return null;
    };
    return findTerminalId(terminalTab.rootPane);
  }

  async function initTerminal() {
    // Check if there's an existing terminal instance in the registry
    const existingInstance = terminalRegistry.get(paneId);
    if (existingInstance) {
      // Reattach existing terminal to new container
      terminal = existingInstance.terminal;
      fitAddon = existingInstance.fitAddon;
      terminalId = existingInstance.terminalId;
      unlisten = existingInstance.unlisten;

      // Move the terminal's DOM element to the new container
      if (terminal.element) {
        // xterm.js manages its own DOM, so we need to manually move it
        // Note: We check only if element exists, not if it has a parent.
        // When component is destroyed, the parent container is removed from DOM,
        // leaving the terminal element orphaned. We can still re-append it.
        // eslint-disable-next-line svelte/no-dom-manipulating
        terminalContainer.appendChild(terminal.element);

        // Force a refresh to redraw the terminal content
        // This is necessary because WebGL context may need to be refreshed after DOM move
        terminal.refresh(0, terminal.rows - 1);
      } else {
        // If element doesn't exist, open the terminal (first time or error recovery)
        terminal.open(terminalContainer);
      }

      // Fit and focus - use same thorough layout wait as initial creation
      document.fonts.ready.then(() => {
        setTimeout(() => {
          requestAnimationFrame(() => {
            requestAnimationFrame(() => {
              fitTerminalToContainer();
              // Force another refresh after fit to ensure content is visible
              if (terminal) {
                terminal.refresh(0, terminal.rows - 1);
              }
              // If size seems wrong (too small), wait more and try again
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
        }, 100); // Match initial creation timing
      });

      // Setup focus tracking for the reattached terminal
      terminal.textarea?.addEventListener('focus', () => {
        isFocused = true;
      });
      terminal.textarea?.addEventListener('blur', () => {
        isFocused = false;
      });

      return;
    }

    // Check if there's an existing PTY for this pane (from store)
    const existingTerminalId = getExistingTerminalId();

    // Lazy load xterm and addons on first use
    if (!xtermLoaded) {
      await import('@xterm/xterm/css/xterm.css');
      xtermLoaded = true;
    }

    // Dynamic imports for xterm modules
    const [{ Terminal }, { FitAddon }, { WebLinksAddon }, { CanvasAddon }, { WebglAddon }] =
      await Promise.all([
        import('@xterm/xterm'),
        import('@xterm/addon-fit'),
        import('@xterm/addon-web-links'),
        import('@xterm/addon-canvas'),
        import('@xterm/addon-webgl'),
      ]);

    // Get current font size from store
    const currentFontSize = get(fontSize);

    terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: 'bar',
      cursorWidth: 2,
      fontFamily: "'JetBrains Mono', 'SF Mono', 'Fira Code', 'Menlo', monospace",
      fontSize: currentFontSize,
      fontWeight: '400',
      fontWeightBold: '500',
      lineHeight: 1.2,
      letterSpacing: 0,
      allowTransparency: true,
      theme: mistTheme,
      scrollback: 10000,
      smoothScrollDuration: 150,
      macOptionIsMeta: true,
      altClickMovesCursor: true,
      // Match macOS Terminal behavior for ED2 (Erase in Display)
      // This prevents blank lines when CLI tools use screen clearing
      scrollOnEraseInDisplay: true,
      // OSC 8 Hyperlink handler - handles explicit hyperlinks from terminal apps
      // See: https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda
      linkHandler: {
        activate: (_event, uri) => {
          // Handle file:// URLs by opening in editor
          if (uri.startsWith('file://')) {
            const filePath = uri.replace('file://', '');
            // Extract line number if present (file:///path/to/file:42)
            const match = filePath.match(/^(.+?):(\d+)(?::(\d+))?$/);
            if (match) {
              const [, path, line, column] = match;
              peekStore.open(path, parseInt(line, 10), column ? parseInt(column, 10) : undefined);
            } else {
              peekStore.open(filePath);
            }
          } else {
            // Open other URLs in browser
            openerService.openUrl(uri);
          }
        },
        allowNonHttpProtocols: true, // Allow file:// protocol
      },
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(
      new WebLinksAddon((_event, uri) => {
        openerService.openUrl(uri);
      })
    );

    // Try WebGL first for GPU-accelerated rendering (better performance)
    // Fall back to Canvas if WebGL is not available or fails
    let webglAddon: InstanceType<typeof WebglAddon> | null = null;
    try {
      webglAddon = new WebglAddon();
      // Handle WebGL context loss - browser may drop context for various reasons (OOM, suspend, etc.)
      webglAddon.onContextLoss(() => {
        console.warn('[Terminal] WebGL context lost, falling back to Canvas renderer');
        webglAddon?.dispose();
        terminal.loadAddon(new CanvasAddon());
      });
      terminal.loadAddon(webglAddon);
    } catch (e) {
      console.warn('[Terminal] WebGL not available, using Canvas renderer:', e);
      terminal.loadAddon(new CanvasAddon());
    }

    // Register file path link provider for peek editor
    terminal.registerLinkProvider(
      createFilePathLinkProvider(terminal, (path, line, column) => {
        peekStore.open(path, line, column);
      })
    );

    // Handle keyboard events for Shift+Enter
    terminal.attachCustomKeyEventHandler((event) => {
      if (event.type !== 'keydown') return true;

      // Handle Shift+Enter
      if (event.key === 'Enter' && event.shiftKey) {
        return false; // Prevent xterm from processing this key
      }

      return true;
    });

    terminal.open(terminalContainer);

    // Wait for layout to be complete before creating PTY
    // This ensures the PTY is created with the correct initial size
    // which is critical for Ink-based apps like Claude Code
    const waitForLayout = (): Promise<void> => {
      return new Promise((resolve) => {
        document.fonts.ready.then(() => {
          // Use longer delay and multiple fit attempts to ensure correct size
          setTimeout(() => {
            requestAnimationFrame(() => {
              requestAnimationFrame(() => {
                if (fitAddon && terminal) {
                  // Use proposeDimensions to calculate, then resize with capped cols
                  const dimensions = fitAddon.proposeDimensions();
                  if (dimensions) {
                    const cols = Math.min(dimensions.cols, MAX_TERMINAL_COLS);
                    const rows = dimensions.rows;
                    terminal.resize(cols, rows);
                  }

                  // If size seems wrong (too small), wait more and try again
                  if (terminal.cols < 40 || terminal.rows < 10) {
                    setTimeout(() => {
                      const dims = fitAddon.proposeDimensions();
                      if (dims) {
                        const cols = Math.min(dims.cols, MAX_TERMINAL_COLS);
                        terminal.resize(cols, dims.rows);
                      }
                      resolve();
                    }, 100);
                    return;
                  }
                }
                resolve();
              });
            });
          }, 100); // Increased from 50ms to 100ms
        });
      });
    };

    // Create PTY or reconnect to existing one
    try {
      if (existingTerminalId !== null) {
        // Reconnect to existing PTY
        terminalId = existingTerminalId;
        // Still need to fit for reconnection
        await waitForLayout();
      } else {
        // Wait for layout before creating PTY
        // waitForLayout already caps cols at MAX_TERMINAL_COLS
        await waitForLayout();

        // Now create PTY with correct initial size
        // Note: cols is already capped by waitForLayout()
        const cols = terminal.cols;
        // Apply row margin to prevent Ink full-height flickering
        const rows = Math.max(terminal.rows - PTY_ROW_MARGIN, 10);

        terminalId = await terminalService.createTerminal(cwd, cols, rows);
        // Store terminal ID in tab store
        tabStore.setTerminalId(tabId, paneId, terminalId);
      }

      terminal?.focus();

      // Synchronized Output Mode (DEC Private Mode 2026)
      // xterm.js doesn't support this natively, so we implement manual buffering
      // to prevent visual glitches from partial frame rendering
      // See: https://github.com/xtermjs/xterm.js/issues/3375
      const SYNC_START = '\x1b[?2026h';
      const SYNC_END = '\x1b[?2026l';
      let syncBuffer = '';
      let inSyncMode = false;
      let pendingWrite = '';
      let writeScheduled = false;
      let syncFrameCount = 0; // Debug: count synced frames

      // Debug flag - set to true to enable sync mode logging
      const DEBUG_SYNC_MODE = false;

      // Flush all pending writes in a single animation frame
      const flushWrites = () => {
        if (pendingWrite && terminal) {
          // If resizing, drop the pending write - it's based on old terminal size
          // Ink apps will redraw after receiving SIGWINCH
          if (isResizing) {
            pendingWrite = '';
            writeScheduled = false;
            return;
          }
          terminal.write(pendingWrite);
          pendingWrite = '';
        }
        writeScheduled = false;
      };

      // Schedule a batched write using requestAnimationFrame
      const scheduleWrite = (data: string) => {
        // If resizing, drop the data - it's based on old terminal size
        // Ink apps will redraw after receiving SIGWINCH
        if (isResizing) {
          return;
        }
        pendingWrite += data;
        if (!writeScheduled) {
          writeScheduled = true;
          requestAnimationFrame(flushWrites);
        }
      };

      // Listen for terminal output
      unlisten = await eventService.listen<TerminalOutput>('terminal-output', (event) => {
        if (event.payload.id === terminalId && terminal) {
          let data = event.payload.data;

          // Process notification escape sequences (OSC 9, OSC 777)
          // This extracts notifications and removes them from the output
          const { output: cleanedData, notifications } =
            notificationService.parseNotifications(data);
          data = cleanedData;

          // Send notifications asynchronously (don't block output)
          if (notifications.length > 0) {
            for (const notification of notifications) {
              notificationService.notify(notification);
            }
          }

          // Process sync mode markers and buffer content
          while (data.length > 0) {
            if (inSyncMode) {
              // Look for sync end marker
              const endIndex = data.indexOf(SYNC_END);
              if (endIndex !== -1) {
                // Add content before end marker to buffer
                syncBuffer += data.substring(0, endIndex);
                // Schedule the entire buffered frame for next animation frame
                scheduleWrite(syncBuffer);
                syncFrameCount++;
                if (DEBUG_SYNC_MODE) {
                  console.log(
                    `[SyncOutput] Frame #${syncFrameCount} flushed (${syncBuffer.length} bytes)`
                  );
                }
                syncBuffer = '';
                inSyncMode = false;
                // Continue processing remaining data
                data = data.substring(endIndex + SYNC_END.length);
              } else {
                // No end marker yet, buffer entire chunk
                syncBuffer += data;
                data = '';
              }
            } else {
              // Look for sync start marker
              const startIndex = data.indexOf(SYNC_START);
              if (startIndex !== -1) {
                // Schedule content before start marker
                if (startIndex > 0) {
                  scheduleWrite(data.substring(0, startIndex));
                }
                inSyncMode = true;
                syncBuffer = '';
                if (DEBUG_SYNC_MODE) {
                  console.log('[SyncOutput] Sync mode started');
                }
                // Continue processing after start marker
                data = data.substring(startIndex + SYNC_START.length);
              } else {
                // No sync markers, schedule write
                scheduleWrite(data);
                data = '';
              }
            }
          }
        }
      });

      // Register instance in registry for preservation across remounts
      if (terminal && fitAddon && terminalId !== null && unlisten) {
        terminalRegistry.register(paneId, {
          terminal,
          fitAddon,
          terminalId,
          unlisten,
        });
      }

      // Send input to PTY
      terminal.onData((data) => {
        if (terminalId !== null) {
          terminalService.writeTerminal(terminalId, data);
        }
      });

      // Handle resize - apply row margin for Ink apps
      terminal.onResize(({ cols, rows }) => {
        if (terminalId !== null) {
          // Apply row margin to prevent Ink full-height flickering
          const ptyRows = Math.max(rows - PTY_ROW_MARGIN, 10);

          // Skip if size hasn't changed (prevents duplicate calls during MAX_TERMINAL_COLS capping)
          if (
            lastSentPtySize &&
            lastSentPtySize.cols === cols &&
            lastSentPtySize.rows === ptyRows
          ) {
            return;
          }

          lastSentPtySize = { cols, rows: ptyRows };
          terminalService.resizeTerminal(terminalId, cols, ptyRows);
        }
      });

      // PTY was created with correct initial size, but trigger terminal-resize
      // to ensure consistent behavior with split panes
      setTimeout(() => {
        window.dispatchEvent(new Event('terminal-resize'));
      }, 100);

      // And another one slightly later to catch any remaining layout changes
      setTimeout(() => {
        window.dispatchEvent(new Event('terminal-resize'));
      }, 250);

      // Track focus state for visual feedback
      terminal.textarea?.addEventListener('focus', () => {
        isFocused = true;
      });
      terminal.textarea?.addEventListener('blur', () => {
        isFocused = false;
      });

      // Handle Shift+Enter to send literal newline (like VSCode)
      // Using capture phase on textarea to intercept before xterm processes it
      terminal.textarea?.addEventListener(
        'keydown',
        (event) => {
          if (event.key === 'Enter' && event.shiftKey) {
            event.preventDefault();
            event.stopPropagation();
            if (terminalId !== null) {
              terminalService.writeTerminal(terminalId, '\n');
            }
          }
        },
        { capture: true }
      );
    } catch (error) {
      console.error('Failed to create terminal:', error);
      terminal.write(`\r\n\x1b[31mError: Failed to create terminal: ${error}\x1b[0m\r\n`);
    }
  }

  let resizeTimeout: ReturnType<typeof setTimeout> | null = null;

  function fitTerminalToContainer() {
    if (!fitAddon || !terminal || !terminalContainer) return;

    // CRITICAL: Guard against 0-size fits
    // When tabs are switched or closed, the container may momentarily have 0 size
    // Sending cols=0/rows=0 to PTY will break Ink-based apps
    const rect = terminalContainer.getBoundingClientRect();
    if (rect.width < 2 || rect.height < 2) {
      return;
    }

    try {
      // Use proposeDimensions() to calculate size without applying,
      // then apply capped size in a single resize to avoid duplicate PTY updates
      const dimensions = fitAddon.proposeDimensions();
      if (!dimensions) return;

      let { cols, rows } = dimensions;

      // Cap terminal width to prevent issues with Ink-based CLI apps
      // Very wide terminals can cause spinner/progress bar rendering glitches
      if (cols > MAX_TERMINAL_COLS) {
        cols = MAX_TERMINAL_COLS;
      }

      // Only resize if size actually changed
      if (terminal.cols !== cols || terminal.rows !== rows) {
        terminal.resize(cols, rows);
      }

      // Note: PTY resize is handled by terminal.onResize handler
      // which applies row margin for Ink-based apps
    } catch (e) {
      console.warn('FitAddon.fit() error:', e);
    }
  }

  /**
   * End resize mode after stability delay
   * This allows output to flow again after resize is complete
   */
  function endResizeMode() {
    isResizing = false;
  }

  /**
   * Schedule the end of resize mode after stability delay
   */
  function scheduleResizeEnd() {
    if (resizeStabilityTimeout) {
      clearTimeout(resizeStabilityTimeout);
    }
    resizeStabilityTimeout = setTimeout(() => {
      requestAnimationFrame(endResizeMode);
    }, RESIZE_STABILITY_DELAY);
  }

  function handleResize() {
    if (terminal) {
      // Start resize buffering to prevent partial frame rendering
      isResizing = true;

      // Debounce resize to avoid rapid calls during window resize
      if (resizeTimeout) {
        clearTimeout(resizeTimeout);
      }
      resizeTimeout = setTimeout(() => {
        // RAF to ensure layout is complete
        requestAnimationFrame(() => {
          fitTerminalToContainer();
          // Schedule buffer clear after resize completes
          scheduleResizeEnd();
        });
      }, 16); // ~1 frame at 60fps
    }
  }

  onMount(() => {
    initTerminal();

    // Initialize notification service for OSC 9/777 notifications
    notificationService.init();

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

    // Also listen for window resize as a fallback
    window.addEventListener('resize', handleResize);

    // Listen for custom terminal-resize event (dispatched when pane sizes change)
    const handleTerminalResize = () => {
      // Start resize buffering
      isResizing = true;
      // Force immediate resize without debounce
      requestAnimationFrame(() => {
        fitTerminalToContainer();
        // Schedule buffer clear after resize completes
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

    // Check if the pane still exists in the store
    // If it does, this is just a remount due to split - preserve the terminal
    const paneStillExists = paneExistsInStore();

    if (paneStillExists) {
      // Pane still exists, terminal will be reattached
      // Don't dispose anything, keep the instance in the registry
      return;
    }

    // Pane is being truly closed - clean up everything
    terminalRegistry.remove(paneId);

    if (unlisten) {
      unlisten();
    }

    if (terminalId !== null) {
      terminalService.closeTerminal(terminalId);
    }

    if (terminal) {
      terminal.dispose();
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
    padding: 4px 8px;
    background: rgba(10, 12, 16, 0.8);
    border-bottom: 1px solid rgba(125, 211, 252, 0.1);
    z-index: 10;
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
    padding: 12px 16px;
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
