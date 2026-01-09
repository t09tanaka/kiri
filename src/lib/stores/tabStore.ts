import { writable, derived, get } from 'svelte/store';

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
  tabs: [
    {
      id: 'terminal-1',
      type: 'terminal',
      title: 'Terminal',
      terminalId: null,
    },
  ],
  activeTabId: 'terminal-1',
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

    reset: () => set(initialState),
  };
}

export const tabStore = createTabStore();

// Derived store for active tab
export const activeTab = derived(tabStore, ($tabStore) => {
  return $tabStore.tabs.find((t) => t.id === $tabStore.activeTabId) || null;
});
