<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import { tabStore } from '@/lib/stores/tabStore';
  import '@xterm/xterm/css/xterm.css';

  interface TerminalOutput {
    id: number;
    data: string;
  }

  interface Props {
    tabId: string;
    cwd?: string | null;
  }

  let { tabId, cwd = null }: Props = $props();

  let terminalContainer: HTMLDivElement;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let terminalId: number | null = null;
  let unlisten: UnlistenFn | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let isFocused = $state(false);

  // Watch for tab activation to focus terminal
  const isActiveTab = $derived($tabStore.activeTabId === tabId);

  $effect(() => {
    if (isActiveTab && terminal) {
      // Small delay to ensure the tab is fully rendered
      requestAnimationFrame(() => {
        terminal?.focus();
      });
    }
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

  async function initTerminal() {
    terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: 'bar',
      cursorWidth: 2,
      fontFamily: "'JetBrains Mono', 'SF Mono', 'Fira Code', 'Geist Mono', monospace",
      fontSize: 13,
      fontWeight: '400',
      fontWeightBold: '500',
      lineHeight: 1.6,
      letterSpacing: 0,
      allowTransparency: true,
      theme: mistTheme,
      scrollback: 10000,
      smoothScrollDuration: 150,
      macOptionIsMeta: true,
      altClickMovesCursor: true,
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());

    // Handle Shift+Enter BEFORE opening terminal
    // This prevents xterm from processing Enter when Shift is held
    terminal.attachCustomKeyEventHandler((event) => {
      if (event.type === 'keydown' && event.key === 'Enter' && event.shiftKey) {
        return false; // Prevent xterm from processing this key
      }
      return true;
    });

    terminal.open(terminalContainer);

    // Delay fit to ensure container is properly sized
    requestAnimationFrame(() => {
      fitAddon?.fit();
      // Auto-focus the terminal after initialization
      terminal?.focus();
    });

    // Create PTY
    try {
      terminalId = await invoke<number>('create_terminal', { cwd });

      // Store terminal ID in tab store
      tabStore.setTerminalId(tabId, terminalId);

      // Listen for terminal output
      unlisten = await listen<TerminalOutput>('terminal-output', (event) => {
        if (event.payload.id === terminalId && terminal) {
          terminal.write(event.payload.data);
        }
      });

      // Send input to PTY
      terminal.onData((data) => {
        if (terminalId !== null) {
          invoke('write_terminal', { id: terminalId, data });
        }
      });

      // Handle resize
      terminal.onResize(({ cols, rows }) => {
        if (terminalId !== null) {
          invoke('resize_terminal', { id: terminalId, cols, rows });
        }
      });

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
              invoke('write_terminal', { id: terminalId, data: '\n' });
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

  function handleResize() {
    if (fitAddon && terminal) {
      // Debounce resize to avoid rapid calls during window resize
      if (resizeTimeout) {
        clearTimeout(resizeTimeout);
      }
      resizeTimeout = setTimeout(() => {
        // Double RAF to ensure layout is complete
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            try {
              fitAddon?.fit();
            } catch (e) {
              console.warn('FitAddon.fit() error:', e);
            }
          });
        });
      }, 50);
    }
  }

  onMount(() => {
    initTerminal();

    // Use ResizeObserver to detect container size changes
    resizeObserver = new ResizeObserver((entries) => {
      // Only resize if the size actually changed meaningfully
      const entry = entries[0];
      if (entry && entry.contentRect.width > 0 && entry.contentRect.height > 0) {
        handleResize();
      }
    });
    resizeObserver.observe(terminalContainer);

    // Also listen for window resize as a fallback
    window.addEventListener('resize', handleResize);
  });

  onDestroy(() => {
    if (resizeTimeout) {
      clearTimeout(resizeTimeout);
    }

    window.removeEventListener('resize', handleResize);

    if (resizeObserver) {
      resizeObserver.disconnect();
    }

    if (unlisten) {
      unlisten();
    }

    if (terminalId !== null) {
      invoke('close_terminal', { id: terminalId });
    }

    if (terminal) {
      terminal.dispose();
    }
  });
</script>

<div class="terminal-wrapper" class:focused={isFocused}>
  <div class="terminal-padding">
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
    width: 100%;
    height: 100%;
    padding: 12px 16px;
    box-sizing: border-box;
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
