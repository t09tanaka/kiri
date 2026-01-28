import { writable, derived, get } from 'svelte/store';
import type { PersistedTab } from '@/lib/services/persistenceService';

// Terminal pane types for split support
export interface TerminalPaneLeaf {
  type: 'terminal';
  id: string; // pane ID (e.g., "pane-1")
  terminalId: number | null;
}

export interface TerminalPaneSplit {
  type: 'split';
  direction: 'horizontal' | 'vertical';
  children: TerminalPane[];
  sizes: number[]; // percentages for each child
}

export type TerminalPane = TerminalPaneLeaf | TerminalPaneSplit;

export interface TerminalTab {
  id: string;
  type: 'terminal';
  title: string;
  rootPane: TerminalPane;
}

export type Tab = TerminalTab;

let nextPaneId = 1;

function generatePaneId(): string {
  return `pane-${nextPaneId++}`;
}

/**
 * Helper: Update terminal ID in a pane tree
 */
function updatePaneTerminalId(
  pane: TerminalPane,
  paneId: string,
  terminalId: number
): TerminalPane {
  if (pane.type === 'terminal') {
    if (pane.id === paneId) {
      return { ...pane, terminalId };
    }
    return pane;
  }
  // Split pane - recurse into children
  return {
    ...pane,
    children: pane.children.map((child) => updatePaneTerminalId(child, paneId, terminalId)),
  };
}

/**
 * Helper: Split a pane in the tree
 */
function splitPaneInTree(
  pane: TerminalPane,
  targetPaneId: string,
  direction: 'horizontal' | 'vertical',
  newPaneId: string
): TerminalPane {
  if (pane.type === 'terminal') {
    if (pane.id === targetPaneId) {
      // Found the target pane, create a split
      return {
        type: 'split',
        direction,
        children: [pane, { type: 'terminal', id: newPaneId, terminalId: null }],
        sizes: [50, 50],
      };
    }
    return pane;
  }
  // Split pane - recurse into children
  return {
    ...pane,
    children: pane.children.map((child) =>
      splitPaneInTree(child, targetPaneId, direction, newPaneId)
    ),
  };
}

/**
 * Helper: Close a pane in the tree
 * Returns null if the entire tree should be removed
 * @internal Exported for testing purposes
 */
export function closePaneInTree(pane: TerminalPane, targetPaneId: string): TerminalPane | null {
  if (pane.type === 'terminal') {
    if (pane.id === targetPaneId) {
      return null; // Remove this pane
    }
    return pane;
  }

  // Split pane - recurse into children
  const newChildren: TerminalPane[] = [];
  for (const child of pane.children) {
    const result = closePaneInTree(child, targetPaneId);
    if (result !== null) {
      newChildren.push(result);
    }
  }

  // If only one child left, return that child (flatten the tree)
  if (newChildren.length === 1) {
    return newChildren[0];
  }

  // If no children left, return null
  if (newChildren.length === 0) {
    return null;
  }

  // Adjust sizes proportionally
  const remainingTotal = newChildren.length;
  const newSizes = newChildren.map(() => 100 / remainingTotal);

  return {
    ...pane,
    children: newChildren,
    sizes: newSizes,
  };
}

/**
 * Helper: Get all pane IDs from a pane tree
 */
export function getAllPaneIds(pane: TerminalPane): string[] {
  if (pane.type === 'terminal') {
    return [pane.id];
  }
  return pane.children.flatMap(getAllPaneIds);
}

/**
 * Helper: Get all terminal IDs from a pane tree
 */
export function getAllTerminalIds(pane: TerminalPane): number[] {
  if (pane.type === 'terminal') {
    return pane.terminalId !== null ? [pane.terminalId] : [];
  }
  return pane.children.flatMap(getAllTerminalIds);
}

/**
 * Helper: Update sizes of a split pane (identified by first child pane ID)
 */
function updatePaneSizesInTree(
  pane: TerminalPane,
  firstChildId: string,
  newSizes: number[]
): TerminalPane {
  if (pane.type === 'terminal') {
    return pane;
  }

  // Check if this split's first child matches
  const firstChild = pane.children[0];
  const firstChildPaneId =
    firstChild.type === 'terminal' ? firstChild.id : getAllPaneIds(firstChild)[0];

  if (firstChildPaneId === firstChildId) {
    return {
      ...pane,
      sizes: newSizes,
    };
  }

  // Recurse into children
  return {
    ...pane,
    children: pane.children.map((child) => updatePaneSizesInTree(child, firstChildId, newSizes)),
  };
}

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

    addTerminalTab: () => {
      update((state) => {
        const terminalCount = state.tabs.length;
        const newTab: TerminalTab = {
          id: generateId(),
          type: 'terminal',
          title: `Terminal ${terminalCount + 1}`,
          rootPane: {
            type: 'terminal',
            id: generatePaneId(),
            terminalId: null,
          },
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

    setTerminalId: (tabId: string, paneId: string, terminalId: number) => {
      update((state) => ({
        ...state,
        tabs: state.tabs.map((t) => {
          if (t.id === tabId && t.type === 'terminal') {
            return {
              ...t,
              rootPane: updatePaneTerminalId(t.rootPane, paneId, terminalId),
            };
          }
          return t;
        }),
      }));
    },

    /**
     * Split a terminal pane horizontally or vertically
     */
    splitPane: (tabId: string, paneId: string, direction: 'horizontal' | 'vertical') => {
      update((state) => ({
        ...state,
        tabs: state.tabs.map((t) => {
          if (t.id === tabId && t.type === 'terminal') {
            return {
              ...t,
              rootPane: splitPaneInTree(t.rootPane, paneId, direction, generatePaneId()),
            };
          }
          return t;
        }),
      }));
    },

    /**
     * Close a terminal pane
     */
    closePane: (tabId: string, paneId: string) => {
      update((state) => ({
        ...state,
        tabs: state.tabs.map((t) => {
          if (t.id === tabId && t.type === 'terminal') {
            const newRootPane = closePaneInTree(t.rootPane, paneId);
            // If no panes left, return original tab (will be handled elsewhere)
            if (!newRootPane) return t;
            return {
              ...t,
              rootPane: newRootPane,
            };
          }
          return t;
        }),
      }));
    },

    /**
     * Update sizes of a split pane
     */
    updatePaneSizes: (tabId: string, firstChildId: string, sizes: number[]) => {
      update((state) => ({
        ...state,
        tabs: state.tabs.map((t) => {
          if (t.id === tabId && t.type === 'terminal') {
            return {
              ...t,
              rootPane: updatePaneSizesInTree(t.rootPane, firstChildId, sizes),
            };
          }
          return t;
        }),
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
      const persistedTabs: PersistedTab[] = state.tabs.map((tab) => ({
        id: tab.id,
        type: 'terminal' as const,
        title: tab.title,
      }));
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
        if (pTab.type === 'terminal') {
          terminalCount++;
          tabs.push({
            id: pTab.id,
            type: 'terminal',
            title: pTab.title || `Terminal ${terminalCount}`,
            rootPane: {
              type: 'terminal',
              id: generatePaneId(),
              terminalId: null,
            },
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

      // Update nextPaneId to avoid ID collisions
      const maxPaneId = tabs
        .flatMap((t) => getAllPaneIds(t.rootPane))
        .map((id) => {
          const match = id.match(/^pane-(\d+)$/);
          // v8 ignore next -- defensive fallback: generatePaneId() always returns pane-{number} format
          return match ? parseInt(match[1], 10) : /* v8 ignore next */ 0;
        })
        .reduce((max, id) => Math.max(max, id), 0);
      nextPaneId = maxPaneId + 1;

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
