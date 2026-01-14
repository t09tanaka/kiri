<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { terminalService } from '@/lib/services/terminalService';
  import { eventService, type UnlistenFn } from '@/lib/services/eventService';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import { CanvasAddon } from '@xterm/addon-canvas';
  import { tabStore, getAllPaneIds, type TerminalTab } from '@/lib/stores/tabStore';
  import { terminalRegistry } from '@/lib/stores/terminalRegistry';
  import { peekStore } from '@/lib/stores/peekStore';
  import { createFilePathLinkProvider } from '@/lib/services/filePathLinkProvider';
  import {
    getSuggestions,
    preloadSuggestions,
    type Suggestion,
  } from '@/lib/services/suggestService';
  import TerminalSuggest from './TerminalSuggest.svelte';
  import '@xterm/xterm/css/xterm.css';

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
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let terminalId: number | null = null;
  let unlisten: UnlistenFn | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let isFocused = $state(false);

  // Suggest feature state
  let currentInput = $state('');
  let suggestions = $state<Suggestion[]>([]);
  let showSuggestions = $state(false);
  let suggestDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  let cursorPosition = $state<{ x: number; y: number } | null>(null);

  // Get the first suggestion for inline display
  const topSuggestion = $derived(suggestions.length > 0 ? suggestions[0] : null);

  // Maximum terminal width to prevent rendering issues with Ink-based apps
  // Wide terminals (140+ cols) can cause spinner glitches due to cursor movement calculations
  const MAX_TERMINAL_COLS = 120;

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

  /**
   * Calculate cursor position in pixels relative to terminal-padding
   */
  function updateCursorPosition() {
    if (!terminal || !terminalPadding || !terminalContainer) {
      cursorPosition = null;
      return;
    }

    try {
      const cursorX = terminal.buffer.active.cursorX;
      const cursorY = terminal.buffer.active.cursorY;

      // Get cell dimensions from the canvas element for accuracy
      const canvas = terminalContainer.querySelector('canvas');
      if (!canvas) {
        cursorPosition = null;
        return;
      }

      // Calculate cell dimensions from canvas size
      const cellWidth = canvas.clientWidth / terminal.cols;
      const cellHeight = canvas.clientHeight / terminal.rows;

      // Use canvas rect directly as reference point
      const canvasRect = canvas.getBoundingClientRect();
      const paddingRect = terminalPadding.getBoundingClientRect();

      // Calculate position relative to terminal-padding
      const offsetX = canvasRect.left - paddingRect.left;
      const offsetY = canvasRect.top - paddingRect.top;

      // Position at the cursor (where next character will appear)
      const x = offsetX + cursorX * cellWidth;
      // Add offset to align with text (canvas renders from top, text needs baseline adjustment)
      const y = offsetY + cursorY * cellHeight + 2;

      cursorPosition = { x, y };
    } catch {
      cursorPosition = null;
    }
  }

  /**
   * Update suggestions based on current input
   */
  async function updateSuggestions() {
    if (!currentInput.trim()) {
      suggestions = [];
      showSuggestions = false;
      cursorPosition = null;
      return;
    }

    try {
      const newSuggestions = await getSuggestions(currentInput, cwd ?? undefined);
      suggestions = newSuggestions;
      showSuggestions = newSuggestions.length > 0;

      if (showSuggestions) {
        updateCursorPosition();
      }
    } catch (error) {
      console.error('Failed to get suggestions:', error);
      suggestions = [];
      showSuggestions = false;
      cursorPosition = null;
    }
  }

  /**
   * Debounced suggestion update
   */
  function debouncedUpdateSuggestions() {
    if (suggestDebounceTimer) {
      clearTimeout(suggestDebounceTimer);
    }
    suggestDebounceTimer = setTimeout(updateSuggestions, 100);
  }

  /**
   * Handle input tracking for suggestions
   */
  function trackInput(data: string) {
    // Handle special characters
    if (data === '\r' || data === '\n') {
      // Enter pressed - clear input and hide suggestions
      currentInput = '';
      showSuggestions = false;
      return;
    }

    if (data === '\x7f' || data === '\b') {
      // Backspace - remove last character
      currentInput = currentInput.slice(0, -1);
      debouncedUpdateSuggestions();
      return;
    }

    if (data === '\x03') {
      // Ctrl+C - clear input
      currentInput = '';
      showSuggestions = false;
      return;
    }

    if (data === '\x1b') {
      // Escape - hide suggestions
      showSuggestions = false;
      return;
    }

    // Skip other control characters and escape sequences
    if (data.charCodeAt(0) < 32 || data.startsWith('\x1b')) {
      return;
    }

    // Regular character - append to input
    currentInput += data;
    debouncedUpdateSuggestions();
  }

  /**
   * Accept the current top suggestion
   */
  function acceptSuggestion() {
    if (!terminal || terminalId === null || !topSuggestion) return;

    const suggestion = topSuggestion.text;

    // Check if we're completing a path (input has space)
    const parts = currentInput.split(/\s+/);
    const isPathCompletion = topSuggestion.kind === 'path' && parts.length > 1;

    if (isPathCompletion) {
      // Only complete the remaining part of the path
      const lastPart = parts[parts.length - 1];
      if (suggestion.toLowerCase().startsWith(lastPart.toLowerCase())) {
        const remaining = suggestion.slice(lastPart.length);
        terminalService.writeTerminal(terminalId, remaining);
        currentInput = parts.slice(0, -1).join(' ') + ' ' + suggestion;
      }
    } else {
      // Complete the command - just add the remaining part
      if (suggestion.toLowerCase().startsWith(currentInput.toLowerCase())) {
        const remaining = suggestion.slice(currentInput.length);
        terminalService.writeTerminal(terminalId, remaining);
        currentInput = suggestion;
      }
    }

    showSuggestions = false;
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
      if (terminal.element && terminal.element.parentElement) {
        // xterm.js manages its own DOM, so we need to manually move it
        // eslint-disable-next-line svelte/no-dom-manipulating
        terminalContainer.appendChild(terminal.element);
      } else {
        // If element doesn't exist, reopen the terminal
        terminal.open(terminalContainer);
      }

      // Fit and focus - use same thorough layout wait as initial creation
      document.fonts.ready.then(() => {
        setTimeout(() => {
          requestAnimationFrame(() => {
            requestAnimationFrame(() => {
              fitTerminalToContainer();
              // If size seems wrong (too small), wait more and try again
              if (terminal && (terminal.cols < 40 || terminal.rows < 10)) {
                setTimeout(() => {
                  fitTerminalToContainer();
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

    terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: 'bar',
      cursorWidth: 2,
      fontFamily: "'JetBrains Mono', 'SF Mono', 'Fira Code', 'Menlo', monospace",
      fontSize: 13,
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
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());
    terminal.loadAddon(new CanvasAddon());

    // Register file path link provider for peek editor
    terminal.registerLinkProvider(
      createFilePathLinkProvider(terminal, (path, line, column) => {
        peekStore.open(path, line, column);
      })
    );

    // Handle keyboard events for suggestions and Shift+Enter
    terminal.attachCustomKeyEventHandler((event) => {
      if (event.type !== 'keydown') return true;

      // Handle Shift+Enter
      if (event.key === 'Enter' && event.shiftKey) {
        return false; // Prevent xterm from processing this key
      }

      // Handle Tab to accept suggestion when visible
      if (event.key === 'Tab' && showSuggestions && topSuggestion) {
        event.preventDefault();
        acceptSuggestion();
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
                  fitAddon.fit();

                  // If size seems wrong (too small), wait more and try again
                  if (terminal.cols < 40 || terminal.rows < 10) {
                    setTimeout(() => {
                      fitAddon.fit();
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
        await waitForLayout();

        // Now create PTY with correct initial size
        // Cap cols to MAX_TERMINAL_COLS to prevent Ink spinner issues
        const cols = Math.min(terminal.cols, MAX_TERMINAL_COLS);
        const rows = terminal.rows;

        // If we capped the cols, resize the terminal to match
        if (terminal.cols > MAX_TERMINAL_COLS) {
          terminal.resize(cols, rows);
        }

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

      // Flush all pending writes in a single animation frame
      const flushWrites = () => {
        if (pendingWrite && terminal) {
          terminal.write(pendingWrite);
          pendingWrite = '';
        }
        writeScheduled = false;
      };

      // Schedule a batched write using requestAnimationFrame
      const scheduleWrite = (data: string) => {
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

      // Send input to PTY and track for suggestions
      terminal.onData((data) => {
        // Track input for suggestions
        trackInput(data);

        if (terminalId !== null) {
          terminalService.writeTerminal(terminalId, data);
        }
      });

      // Handle resize
      terminal.onResize(({ cols, rows }) => {
        if (terminalId !== null) {
          terminalService.resizeTerminal(terminalId, cols, rows);
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
      fitAddon.fit();

      // Cap terminal width to prevent issues with Ink-based CLI apps
      // Very wide terminals can cause spinner/progress bar rendering glitches
      let cols = terminal.cols;
      const rows = terminal.rows;

      if (cols > MAX_TERMINAL_COLS) {
        cols = MAX_TERMINAL_COLS;
        terminal.resize(cols, rows);
      }

      // Explicitly send resize to PTY after fit
      // This ensures Ink-based apps get the correct size immediately
      if (terminalId !== null && cols > 0 && rows > 0) {
        terminalService.resizeTerminal(terminalId, cols, rows);
      }
    } catch (e) {
      console.warn('FitAddon.fit() error:', e);
    }
  }

  function handleResize() {
    if (terminal) {
      // Debounce resize to avoid rapid calls during window resize
      if (resizeTimeout) {
        clearTimeout(resizeTimeout);
      }
      resizeTimeout = setTimeout(() => {
        // RAF to ensure layout is complete
        requestAnimationFrame(() => {
          fitTerminalToContainer();
        });
      }, 16); // ~1 frame at 60fps
    }
  }

  onMount(() => {
    initTerminal();

    // Preload suggestions in the background
    preloadSuggestions();

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
      // Force immediate resize without debounce
      requestAnimationFrame(() => {
        fitTerminalToContainer();
      });
    };
    window.addEventListener('terminal-resize', handleTerminalResize);

    return () => {
      window.removeEventListener('terminal-resize', handleTerminalResize);
    };
  });

  onDestroy(() => {
    if (resizeTimeout) {
      clearTimeout(resizeTimeout);
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
    <TerminalSuggest
      suggestion={topSuggestion}
      {currentInput}
      visible={showSuggestions}
      {cursorPosition}
    />
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
