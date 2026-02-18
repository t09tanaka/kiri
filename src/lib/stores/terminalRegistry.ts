import { writable, get } from 'svelte/store';
import type { Terminal } from '@xterm/xterm';
import type { FitAddon } from '@xterm/addon-fit';
import type { UnlistenFn } from '@tauri-apps/api/event';

/**
 * Terminal instance data stored in registry
 */
export interface TerminalInstance {
  terminal: Terminal;
  fitAddon: FitAddon;
  terminalId: number;
  unlisten: UnlistenFn;
}

/**
 * Registry to preserve terminal instances across component remounts.
 * When a terminal pane is split, the component hierarchy changes and Svelte
 * destroys/recreates Terminal components. This registry allows us to preserve
 * the actual terminal instances and reattach them to new DOM containers.
 */
function createTerminalRegistry() {
  const { subscribe, update } = writable<Map<string, TerminalInstance>>(new Map());

  return {
    subscribe,

    /**
     * Register a terminal instance for a pane
     */
    register: (paneId: string, instance: TerminalInstance) => {
      update((map) => {
        map.set(paneId, instance);
        return new Map(map);
      });
    },

    /**
     * Get a terminal instance by pane ID
     */
    get: (paneId: string): TerminalInstance | undefined => {
      const map = get({ subscribe });
      return map.get(paneId);
    },

    /**
     * Check if a terminal instance exists for a pane
     */
    has: (paneId: string): boolean => {
      const map = get({ subscribe });
      return map.has(paneId);
    },

    /**
     * Remove and dispose a terminal instance
     */
    remove: (paneId: string): TerminalInstance | undefined => {
      let instance: TerminalInstance | undefined;
      update((map) => {
        instance = map.get(paneId);
        map.delete(paneId);
        return new Map(map);
      });
      return instance;
    },

    /**
     * Detach a terminal instance (remove from registry but don't dispose)
     * Used when component is destroyed but terminal should be preserved
     */
    detach: (paneId: string): TerminalInstance | undefined => {
      let instance: TerminalInstance | undefined;
      update((map) => {
        instance = map.get(paneId);
        // Keep in registry but mark as detached by returning the instance
        return map;
      });
      return instance;
    },

    /**
     * Clear all terminal instances, disposing each one.
     * Used when switching projects to clean up all PTY sessions.
     * Returns the terminal IDs that were cleared (for PTY cleanup).
     */
    clearAll: (): number[] => {
      const terminalIds: number[] = [];
      update((map) => {
        for (const [, instance] of map) {
          terminalIds.push(instance.terminalId);
          instance.unlisten();
          try {
            instance.terminal.dispose();
          } catch {
            // WebGL/Canvas addon may throw during dispose if DOM is already detached
          }
        }
        return new Map();
      });
      return terminalIds;
    },
  };
}

export const terminalRegistry = createTerminalRegistry();
