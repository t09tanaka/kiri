import { writable } from 'svelte/store';

export type PaneColor = 'sky' | 'iris' | 'jade' | 'amber' | 'coral' | 'rose';

export interface TerminalPaneLeaf {
  type: 'terminal';
  id: string;
  terminalId: number | null;
  cwd?: string | null;
  name?: string;
  color?: PaneColor;
}

export interface TerminalPaneSplit {
  type: 'split';
  id: string;
  direction: 'horizontal' | 'vertical';
  children: TerminalPane[];
  sizes: number[];
}

export type TerminalPane = TerminalPaneLeaf | TerminalPaneSplit;

let nextPaneId = 1;
let nextSplitId = 1;

function generatePaneId(): string {
  return `pane-${nextPaneId++}`;
}

function generateSplitId(): string {
  return `split-${nextSplitId++}`;
}

function createInitialPane(): TerminalPaneLeaf {
  return {
    type: 'terminal',
    id: generatePaneId(),
    terminalId: null,
  };
}

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
  return {
    ...pane,
    children: pane.children.map((child) => updatePaneTerminalId(child, paneId, terminalId)),
  };
}

/**
 * @internal Exported for testing. Replaces the leaf with id `paneId`'s
 * `name` / `color`. Each field follows a three-state convention:
 *   - `undefined`  → leave alone
 *   - `null`       → remove the key from the leaf
 *   - a value      → set the leaf to that value
 *
 * Splits are walked unchanged. The returned tree is a new instance only
 * when something actually changed.
 */
export function setPaneLabelInTree(
  pane: TerminalPane,
  paneId: string,
  opts: { name?: string | null; color?: PaneColor | null }
): TerminalPane {
  if (pane.type === 'terminal') {
    if (pane.id !== paneId) return pane;
    const next: TerminalPaneLeaf = { ...pane };
    if ('name' in opts) {
      if (opts.name === null) delete next.name;
      else if (typeof opts.name === 'string') next.name = opts.name;
    }
    if ('color' in opts) {
      if (opts.color === null) delete next.color;
      else if (opts.color !== undefined) next.color = opts.color;
    }
    return next;
  }
  return {
    ...pane,
    children: pane.children.map((child) => setPaneLabelInTree(child, paneId, opts)),
  };
}

function splitPaneInTree(
  pane: TerminalPane,
  targetPaneId: string,
  direction: 'horizontal' | 'vertical',
  newPaneId: string,
  newPaneOpts: { name?: string; color?: PaneColor } = {}
): TerminalPane {
  if (pane.type === 'terminal') {
    if (pane.id === targetPaneId) {
      return {
        type: 'split',
        id: generateSplitId(),
        direction,
        children: [pane, { type: 'terminal', id: newPaneId, terminalId: null, ...newPaneOpts }],
        sizes: [50, 50],
      };
    }
    return pane;
  }

  if (pane.direction === direction) {
    const targetIndex = pane.children.findIndex(
      (child) => child.type === 'terminal' && child.id === targetPaneId
    );

    if (targetIndex !== -1) {
      const newChildren = [...pane.children];
      newChildren.splice(targetIndex + 1, 0, {
        type: 'terminal',
        id: newPaneId,
        terminalId: null,
        ...newPaneOpts,
      });

      const equalSize = 100 / newChildren.length;
      const newSizes = newChildren.map(() => equalSize);

      return {
        ...pane,
        children: newChildren,
        sizes: newSizes,
      };
    }
  }

  return {
    ...pane,
    children: pane.children.map((child) =>
      splitPaneInTree(child, targetPaneId, direction, newPaneId, newPaneOpts)
    ),
  };
}

/**
 * @internal Exported for testing purposes
 */
export function closePaneInTree(pane: TerminalPane, targetPaneId: string): TerminalPane | null {
  if (pane.type === 'terminal') {
    if (pane.id === targetPaneId) {
      return null;
    }
    return pane;
  }

  const newChildren: TerminalPane[] = [];
  const keptSizes: number[] = [];
  for (let i = 0; i < pane.children.length; i++) {
    const result = closePaneInTree(pane.children[i], targetPaneId);
    if (result !== null) {
      newChildren.push(result);
      keptSizes.push(pane.sizes[i]);
    }
  }

  if (newChildren.length === 1) {
    return newChildren[0];
  }

  if (newChildren.length === 0) {
    return null;
  }

  const totalKept = keptSizes.reduce((sum, s) => sum + s, 0);
  const newSizes = keptSizes.map((s) => (s / totalKept) * 100);

  return {
    ...pane,
    children: newChildren,
    sizes: newSizes,
  };
}

export function getAllPaneIds(pane: TerminalPane): string[] {
  if (pane.type === 'terminal') {
    return [pane.id];
  }
  return pane.children.flatMap(getAllPaneIds);
}

export function getAllSplitIds(pane: TerminalPane): string[] {
  if (pane.type === 'terminal') {
    return [];
  }
  return [pane.id, ...pane.children.flatMap(getAllSplitIds)];
}

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

export function getAllTerminalIds(pane: TerminalPane): number[] {
  if (pane.type === 'terminal') {
    return pane.terminalId !== null ? [pane.terminalId] : [];
  }
  return pane.children.flatMap(getAllTerminalIds);
}

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

function updatePaneSizesInTree(
  pane: TerminalPane,
  splitId: string,
  newSizes: number[]
): TerminalPane {
  if (pane.type === 'terminal') {
    return pane;
  }

  if (pane.id === splitId) {
    return {
      ...pane,
      sizes: newSizes,
    };
  }

  return {
    ...pane,
    children: pane.children.map((child) => updatePaneSizesInTree(child, splitId, newSizes)),
  };
}

export interface TerminalState {
  rootPane: TerminalPane | null;
}

/** Snapshot shape exposed via `terminalStore.snapshot()` — augments TerminalState with the collapsed map. */
export interface TerminalSnapshot extends TerminalState {
  collapsedByPaneId: Map<string, boolean>;
}

const initialState: TerminalState = {
  rootPane: null,
};

/**
 * Tracks which panes have their shortcut bar collapsed/minimized.
 * Module-level state mirroring the existing `nextPaneId` / `nextSplitId` pattern.
 * Not reactive on its own — subscribers are notified via the writable's update().
 * Only `true` entries are stored; `false` is the implicit default.
 */
const collapsedByPaneId = new Map<string, boolean>();

function createTerminalStore() {
  const { subscribe, set, update } = writable<TerminalState>(initialState);

  /** Force-notify subscribers without changing the tree shape. */
  function notify() {
    update((state) => state);
  }

  return {
    subscribe,

    /**
     * Initialize a fresh single-terminal pane tree.
     * No-op if a rootPane already exists (use reset() first to replace).
     */
    init: () => {
      update((state) => {
        if (state.rootPane) return state;
        return { rootPane: createInitialPane() };
      });
    },

    setTerminalId: (paneId: string, terminalId: number) => {
      update((state) => {
        if (!state.rootPane) return state;
        return { rootPane: updatePaneTerminalId(state.rootPane, paneId, terminalId) };
      });
    },

    splitPane: (
      paneId: string,
      direction: 'horizontal' | 'vertical',
      opts: { name?: string; color?: PaneColor } = {}
    ): string => {
      const newPaneId = generatePaneId();
      update((state) => {
        if (!state.rootPane) return state;
        return {
          rootPane: splitPaneInTree(state.rootPane, paneId, direction, newPaneId, opts),
        };
      });
      return newPaneId;
    },

    /**
     * Update an existing pane's label. `name`/`color` follow the same
     * three-state convention as {@link setPaneLabelInTree}: `undefined`
     * leaves the field alone, `null` clears it, a value installs it.
     *
     * No-op when no leaf matches `paneId` — keeps the store quiet about
     * panes that disappeared mid-flight.
     */
    setPaneLabel: (paneId: string, opts: { name?: string | null; color?: PaneColor | null }) => {
      update((state) => {
        if (!state.rootPane) return state;
        return { rootPane: setPaneLabelInTree(state.rootPane, paneId, opts) };
      });
    },

    closePane: (paneId: string) => {
      update((state) => {
        if (!state.rootPane) return state;
        const beforeIds = new Set(getAllPaneIds(state.rootPane));
        const newRootPane = closePaneInTree(state.rootPane, paneId);
        const afterIds = new Set(newRootPane ? getAllPaneIds(newRootPane) : []);
        // Drop collapsed entries for every pane that disappeared from the tree
        // (the target plus any descendants if a split branch collapsed).
        for (const id of beforeIds) {
          if (!afterIds.has(id)) {
            collapsedByPaneId.delete(id);
          }
        }
        return { rootPane: newRootPane };
      });
    },

    updatePaneSizes: (splitId: string, sizes: number[]) => {
      update((state) => {
        if (!state.rootPane) return state;
        return { rootPane: updatePaneSizesInTree(state.rootPane, splitId, sizes) };
      });
    },

    /**
     * Synchronous read of the current state. Used by the CLI bridge
     * which needs to look up pane → terminal id mappings on demand.
     * Returns a defensive copy of `collapsedByPaneId` so callers can
     * mutate the snapshot freely without affecting store state.
     */
    snapshot: (): TerminalSnapshot => {
      let state: TerminalState = initialState;
      const unsub = subscribe((s) => {
        state = s;
      });
      unsub();
      return { ...state, collapsedByPaneId: new Map(collapsedByPaneId) };
    },

    /** Returns whether the shortcut bar for `paneId` is currently collapsed. */
    isCollapsed: (paneId: string): boolean => collapsedByPaneId.get(paneId) ?? false,

    /**
     * Set the collapsed state for `paneId`. `true` adds an entry, `false`
     * deletes it (keeping the map sparse). Notifies subscribers either way.
     */
    setCollapsed: (paneId: string, value: boolean): void => {
      if (value) {
        collapsedByPaneId.set(paneId, true);
      } else {
        collapsedByPaneId.delete(paneId);
      }
      notify();
    },

    /** Flip the collapsed state for `paneId`. */
    toggleCollapsed: (paneId: string): void => {
      const current = collapsedByPaneId.get(paneId) ?? false;
      if (current) {
        collapsedByPaneId.delete(paneId);
      } else {
        collapsedByPaneId.set(paneId, true);
      }
      notify();
    },

    /**
     * Index (depth-first order) of `paneId` in the current tree, or -1.
     */
    indexOf: (paneId: string): number => {
      let state: TerminalState = initialState;
      const unsub = subscribe((s) => {
        state = s;
      });
      unsub();
      if (!state.rootPane) return -1;
      return getAllPaneIds(state.rootPane).indexOf(paneId);
    },

    /**
     * Look up the physical PTY id behind a logical pane id, or null.
     */
    terminalIdFor: (paneId: string): number | null => {
      let state: TerminalState = initialState;
      const unsub = subscribe((s) => {
        state = s;
      });
      unsub();
      if (!state.rootPane) return null;
      return getPaneTerminalIdMap(state.rootPane).get(paneId) ?? null;
    },

    reset: () => {
      collapsedByPaneId.clear();
      set(initialState);
    },
  };
}

export const terminalStore = createTerminalStore();
