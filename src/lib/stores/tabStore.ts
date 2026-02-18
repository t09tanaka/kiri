import { writable, derived, get } from 'svelte/store';

// Terminal pane types for split support
export interface TerminalPaneLeaf {
  type: 'terminal';
  id: string; // pane ID (e.g., "pane-1")
  terminalId: number | null;
  cwd?: string | null;
}

export interface TerminalPaneSplit {
  type: 'split';
  id: string; // unique split ID (e.g., "split-1")
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
let nextSplitId = 1;

function generatePaneId(): string {
  return `pane-${nextPaneId++}`;
}

function generateSplitId(): string {
  return `split-${nextSplitId++}`;
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
 * If splitting in the same direction as the parent split, add as sibling.
 * If splitting in a different direction, create a nested split.
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
        id: generateSplitId(),
        direction,
        children: [pane, { type: 'terminal', id: newPaneId, terminalId: null }],
        sizes: [50, 50],
      };
    }
    return pane;
  }

  // Split pane - check if we should add sibling or nest
  if (pane.direction === direction) {
    // Same direction: check if target is a direct child
    const targetIndex = pane.children.findIndex(
      (child) => child.type === 'terminal' && child.id === targetPaneId
    );

    if (targetIndex !== -1) {
      // Target is a direct child, add new pane as sibling after the target
      const newChildren = [...pane.children];
      newChildren.splice(targetIndex + 1, 0, {
        type: 'terminal',
        id: newPaneId,
        terminalId: null,
      });

      // Distribute sizes equally among all children
      const equalSize = 100 / newChildren.length;
      const newSizes = newChildren.map(() => equalSize);

      return {
        ...pane,
        children: newChildren,
        sizes: newSizes,
      };
    }
  }

  // Recurse into children for nested splits or different direction
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

  // Split pane - recurse into children, tracking original sizes
  const newChildren: TerminalPane[] = [];
  const keptSizes: number[] = [];
  for (let i = 0; i < pane.children.length; i++) {
    const result = closePaneInTree(pane.children[i], targetPaneId);
    if (result !== null) {
      newChildren.push(result);
      keptSizes.push(pane.sizes[i]);
    }
  }

  // If only one child left, return just that child (flatten the tree)
  if (newChildren.length === 1) {
    return newChildren[0];
  }

  // If no children left, return null
  if (newChildren.length === 0) {
    return null;
  }

  // Adjust sizes proportionally based on original sizes
  const totalKept = keptSizes.reduce((sum, s) => sum + s, 0);
  const newSizes = keptSizes.map((s) => (s / totalKept) * 100);

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
 * Helper: Get all split IDs from a pane tree
 */
export function getAllSplitIds(pane: TerminalPane): string[] {
  if (pane.type === 'terminal') {
    return [];
  }
  return [pane.id, ...pane.children.flatMap(getAllSplitIds)];
}

/**
 * Helper: Get the first terminal ID from a pane tree (depth-first)
 */
export function getFirstTerminalId(pane: TerminalPane): number | null {
  if (pane.type === 'terminal') {
    return pane.terminalId;
  }
  for (const child of pane.children) {
    const id = getFirstTerminalId(child);
    if (id !== null) return id;
  }
  return null;
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
 * Get a mapping of paneId -> terminalId for all terminal panes
 */
export function getPaneTerminalIdMap(pane: TerminalPane): Map<string, number> {
  const map = new Map<string, number>();
  if (pane.type === 'terminal') {
    if (pane.terminalId !== null) {
      map.set(pane.id, pane.terminalId);
    }
  } else {
    for (const child of pane.children) {
      const childMap = getPaneTerminalIdMap(child);
      childMap.forEach((v, k) => map.set(k, v));
    }
  }
  return map;
}

/**
 * Helper: Update sizes of a split pane (identified by split ID)
 */
function updatePaneSizesInTree(
  pane: TerminalPane,
  splitId: string,
  newSizes: number[]
): TerminalPane {
  if (pane.type === 'terminal') {
    return pane;
  }

  // Check if this split matches by ID
  if (pane.id === splitId) {
    return {
      ...pane,
      sizes: newSizes,
    };
  }

  // Recurse into children
  return {
    ...pane,
    children: pane.children.map((child) => updatePaneSizesInTree(child, splitId, newSizes)),
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
        const newTab: TerminalTab = {
          id: generateId(),
          type: 'terminal',
          title: 'Terminal',
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
    updatePaneSizes: (tabId: string, splitId: string, sizes: number[]) => {
      update((state) => ({
        ...state,
        tabs: state.tabs.map((t) => {
          if (t.id === tabId && t.type === 'terminal') {
            return {
              ...t,
              rootPane: updatePaneSizesInTree(t.rootPane, splitId, sizes),
            };
          }
          return t;
        }),
      }));
    },

    updateTabTitle: (tabId: string, title: string) => {
      update((state) => ({
        ...state,
        tabs: state.tabs.map((t) => (t.id === tabId ? { ...t, title } : t)),
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
