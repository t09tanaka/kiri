import { writable, derived, get } from 'svelte/store';
import type { PersistedTab } from '@/lib/services/persistenceService';

export interface EditorTab {
  id: string;
  type: 'editor';
  filePath: string;
  modified: boolean;
}

export interface TerminalTab {
  id: string;
  type: 'terminal';
  title: string;
  terminalId: number | null;
}

export type Tab = EditorTab | TerminalTab;

export interface TabState {
  tabs: Tab[];
  activeTabId: string | null;
}

let nextId = 1;

function generateId(): string {
  return `tab-${nextId++}`;
}

const initialState: TabState = {
  tabs: [],
  activeTabId: null,
};

function createTabStore() {
  const { subscribe, set, update } = writable<TabState>(initialState);

  return {
    subscribe,

    addEditorTab: (filePath: string) => {
      update((state) => {
        // Check if file is already open
        const existing = state.tabs.find((t) => t.type === 'editor' && t.filePath === filePath);
        if (existing) {
          return { ...state, activeTabId: existing.id };
        }

        const newTab: EditorTab = {
          id: generateId(),
          type: 'editor',
          filePath,
          modified: false,
        };

        return {
          tabs: [...state.tabs, newTab],
          activeTabId: newTab.id,
        };
      });
    },

    addTerminalTab: () => {
      update((state) => {
        const terminalCount = state.tabs.filter((t) => t.type === 'terminal').length;
        const newTab: TerminalTab = {
          id: generateId(),
          type: 'terminal',
          title: `Terminal ${terminalCount + 1}`,
          terminalId: null,
        };

        return {
          tabs: [...state.tabs, newTab],
          activeTabId: newTab.id,
        };
      });
    },

    closeTab: (id: string) => {
      update((state) => {
        const index = state.tabs.findIndex((t) => t.id === id);
        if (index === -1) return state;

        const newTabs = state.tabs.filter((t) => t.id !== id);

        // If closing active tab, switch to adjacent tab
        let newActiveId = state.activeTabId;
        if (state.activeTabId === id) {
          if (newTabs.length === 0) {
            newActiveId = null;
          } else if (index >= newTabs.length) {
            newActiveId = newTabs[newTabs.length - 1].id;
          } else {
            newActiveId = newTabs[index].id;
          }
        }

        return { tabs: newTabs, activeTabId: newActiveId };
      });
    },

    setActiveTab: (id: string) => {
      update((state) => ({ ...state, activeTabId: id }));
    },

    setModified: (id: string, modified: boolean) => {
      update((state) => ({
        ...state,
        tabs: state.tabs.map((t) => (t.id === id && t.type === 'editor' ? { ...t, modified } : t)),
      }));
    },

    setTerminalId: (tabId: string, terminalId: number) => {
      update((state) => ({
        ...state,
        tabs: state.tabs.map((t) =>
          t.id === tabId && t.type === 'terminal' ? { ...t, terminalId } : t
        ),
      }));
    },

    getActiveTab: () => {
      const state = get({ subscribe });
      return state.tabs.find((t) => t.id === state.activeTabId) || null;
    },

    /**
     * Get current state for persistence (convert to PersistedTab format)
     */
    getStateForPersistence: (): { tabs: PersistedTab[]; activeTabId: string | null } => {
      const state = get({ subscribe });
      const persistedTabs: PersistedTab[] = state.tabs.map((tab) => {
        if (tab.type === 'editor') {
          return {
            id: tab.id,
            type: 'editor' as const,
            filePath: tab.filePath,
          };
        } else {
          return {
            id: tab.id,
            type: 'terminal' as const,
            title: tab.title,
          };
        }
      });
      return {
        tabs: persistedTabs,
        activeTabId: state.activeTabId,
      };
    },

    /**
     * Restore state from persistence
     */
    restoreState: (persistedTabs: PersistedTab[], activeTabId: string | null) => {
      const tabs: Tab[] = [];
      let terminalCount = 0;

      for (const pTab of persistedTabs) {
        if (pTab.type === 'editor' && pTab.filePath) {
          tabs.push({
            id: pTab.id,
            type: 'editor',
            filePath: pTab.filePath,
            modified: false,
          });
        } else if (pTab.type === 'terminal') {
          terminalCount++;
          tabs.push({
            id: pTab.id,
            type: 'terminal',
            title: pTab.title || `Terminal ${terminalCount}`,
            terminalId: null, // Will be assigned when terminal initializes
          });
        }
      }

      // Update nextId to avoid ID collisions
      const maxNumericId = tabs
        .map((t) => {
          const match = t.id.match(/^tab-(\d+)$/);
          return match ? parseInt(match[1], 10) : 0;
        })
        .reduce((max, id) => Math.max(max, id), 0);
      nextId = maxNumericId + 1;

      set({
        tabs,
        activeTabId: activeTabId && tabs.find((t) => t.id === activeTabId) ? activeTabId : null,
      });
    },

    reset: () => set(initialState),
  };
}

export const tabStore = createTabStore();

// Derived store for active tab
export const activeTab = derived(tabStore, ($tabStore) => {
  return $tabStore.tabs.find((t) => t.id === $tabStore.activeTabId) || null;
});
