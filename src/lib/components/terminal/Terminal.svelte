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
  }

  let { tabId }: Props = $props();

  let terminalContainer: HTMLDivElement;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let terminalId: number | null = null;
  let unlisten: UnlistenFn | null = null;

  async function initTerminal() {
    terminal = new Terminal({
      cursorBlink: true,
      fontFamily: 'JetBrains Mono, Menlo, Monaco, monospace',
      fontSize: 14,
      lineHeight: 1.2,
      theme: {
        background: '#1e1e1e',
        foreground: '#cccccc',
        cursor: '#cccccc',
        cursorAccent: '#1e1e1e',
        selectionBackground: '#264f78',
        black: '#1e1e1e',
        red: '#f44747',
        green: '#6a9955',
        yellow: '#dcdcaa',
        blue: '#569cd6',
        magenta: '#c586c0',
        cyan: '#4ec9b0',
        white: '#d4d4d4',
        brightBlack: '#808080',
        brightRed: '#f44747',
        brightGreen: '#6a9955',
        brightYellow: '#dcdcaa',
        brightBlue: '#569cd6',
        brightMagenta: '#c586c0',
        brightCyan: '#4ec9b0',
        brightWhite: '#ffffff',
      },
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());

    terminal.open(terminalContainer);
    fitAddon.fit();

    // Create PTY
    try {
      terminalId = await invoke<number>('create_terminal', { cwd: null });

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
    } catch (error) {
      console.error('Failed to create terminal:', error);
      terminal.write(`\r\nError: Failed to create terminal: ${error}\r\n`);
    }
  }

  function handleResize() {
    if (fitAddon) {
      fitAddon.fit();
    }
  }

  onMount(() => {
    initTerminal();
    window.addEventListener('resize', handleResize);
  });

  onDestroy(() => {
    window.removeEventListener('resize', handleResize);

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

<div class="terminal-wrapper" bind:this={terminalContainer}></div>

<style>
  .terminal-wrapper {
    width: 100%;
    height: 100%;
    background-color: var(--bg-primary);
  }

  .terminal-wrapper :global(.xterm) {
    height: 100%;
    padding: 8px;
  }

  .terminal-wrapper :global(.xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
