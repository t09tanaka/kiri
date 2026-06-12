import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  terminalStore,
  closePaneInTree,
  getAllPaneIds,
  getAllLeaves,
  findPaneLeaf,
  getAllSplitIds,
  getFirstTerminalId,
  getAllTerminalIds,
  getPaneTerminalIdMap,
  type TerminalPane,
  type TerminalPaneLeaf,
  type TerminalPaneSplit,
} from './terminalStore';

describe('terminalStore', () => {
  beforeEach(() => {
    terminalStore.reset();
  });

  describe('initial state', () => {
    it('should have null rootPane', () => {
      const state = get(terminalStore);
      expect(state.rootPane).toBeNull();
    });
  });

  describe('init', () => {
    it('should create a single terminal pane as root', () => {
      terminalStore.init();
      const state = get(terminalStore);
      expect(state.rootPane).not.toBeNull();
      expect(state.rootPane?.type).toBe('terminal');
      if (state.rootPane?.type === 'terminal') {
        expect(state.rootPane.terminalId).toBeNull();
        expect(state.rootPane.id).toMatch(/^pane-\d+$/);
      }
    });

    it('should be a no-op when a rootPane already exists', () => {
      terminalStore.init();
      const before = get(terminalStore).rootPane;
      terminalStore.init();
      const after = get(terminalStore).rootPane;
      expect(after).toBe(before);
    });
  });

  describe('setTerminalId', () => {
    it('should set terminalId on the matching pane', () => {
      terminalStore.init();
      const paneId = (get(terminalStore).rootPane as { id: string }).id;
      terminalStore.setTerminalId(paneId, 42);
      const root = get(terminalStore).rootPane;
      expect(root?.type).toBe('terminal');
      if (root?.type === 'terminal') {
        expect(root.terminalId).toBe(42);
      }
    });

    it('should be a no-op when rootPane is null', () => {
      terminalStore.setTerminalId('pane-x', 1);
      expect(get(terminalStore).rootPane).toBeNull();
    });
  });

  describe('splitPane', () => {
    it('should turn a single terminal pane into a split with two children', () => {
      terminalStore.init();
      const rootPaneId = (get(terminalStore).rootPane as { id: string }).id;

      terminalStore.splitPane(rootPaneId, 'vertical');

      const root = get(terminalStore).rootPane;
      expect(root?.type).toBe('split');
      if (root?.type === 'split') {
        expect(root.direction).toBe('vertical');
        expect(root.children).toHaveLength(2);
        expect(root.sizes).toEqual([50, 50]);
      }
    });

    it('should add a sibling when splitting again in the same direction', () => {
      terminalStore.init();
      const firstPaneId = (get(terminalStore).rootPane as { id: string }).id;
      terminalStore.splitPane(firstPaneId, 'vertical');

      const after = get(terminalStore).rootPane;
      const secondPaneId =
        after?.type === 'split' && after.children[1].type === 'terminal'
          ? after.children[1].id
          : '';

      terminalStore.splitPane(secondPaneId, 'vertical');

      const root = get(terminalStore).rootPane;
      expect(root?.type).toBe('split');
      if (root?.type === 'split') {
        expect(root.children).toHaveLength(3);
        expect(root.sizes.every((s) => Math.abs(s - 100 / 3) < 0.01)).toBe(true);
      }
    });

    it('attaches name to the new pane when opts.name is given', () => {
      terminalStore.init();
      const state1 = get(terminalStore);
      const rootPaneId = (state1.rootPane as TerminalPaneLeaf).id;
      const newId = terminalStore.splitPane(rootPaneId, 'vertical', { name: 'build' });
      const state2 = get(terminalStore);
      const ids = getAllPaneIds(state2.rootPane!);
      expect(ids).toContain(newId);
      const split = state2.rootPane as TerminalPaneSplit;
      const newLeaf = split.children.find(
        (c) => c.type === 'terminal' && c.id === newId
      ) as TerminalPaneLeaf;
      expect(newLeaf.name).toBe('build');
      expect(newLeaf.color).toBeUndefined();
    });

    it('attaches color to the new pane when opts.color is given', () => {
      terminalStore.init();
      const state1 = get(terminalStore);
      const rootPaneId = (state1.rootPane as TerminalPaneLeaf).id;
      const newId = terminalStore.splitPane(rootPaneId, 'vertical', { color: 'jade' });
      const state2 = get(terminalStore);
      const split = state2.rootPane as TerminalPaneSplit;
      const newLeaf = split.children.find(
        (c) => c.type === 'terminal' && c.id === newId
      ) as TerminalPaneLeaf;
      expect(newLeaf.color).toBe('jade');
      expect(newLeaf.name).toBeUndefined();
    });

    it('leaves the original pane unlabeled even when child has name/color', () => {
      terminalStore.init();
      const state1 = get(terminalStore);
      const rootPaneId = (state1.rootPane as TerminalPaneLeaf).id;
      terminalStore.splitPane(rootPaneId, 'vertical', { name: 'build', color: 'coral' });
      const state2 = get(terminalStore);
      const split = state2.rootPane as TerminalPaneSplit;
      const original = split.children.find(
        (c) => c.type === 'terminal' && c.id === rootPaneId
      ) as TerminalPaneLeaf;
      expect(original.name).toBeUndefined();
      expect(original.color).toBeUndefined();
    });
  });

  describe('setPaneLabel', () => {
    it('sets name on the matching leaf', () => {
      terminalStore.init();
      const leaf = get(terminalStore).rootPane as TerminalPaneLeaf;
      terminalStore.setPaneLabel(leaf.id, { name: 'agent' });
      const next = get(terminalStore).rootPane as TerminalPaneLeaf;
      expect(next.name).toBe('agent');
      expect(next.color).toBeUndefined();
    });

    it('sets color on the matching leaf', () => {
      terminalStore.init();
      const leaf = get(terminalStore).rootPane as TerminalPaneLeaf;
      terminalStore.setPaneLabel(leaf.id, { color: 'jade' });
      const next = get(terminalStore).rootPane as TerminalPaneLeaf;
      expect(next.color).toBe('jade');
      expect(next.name).toBeUndefined();
    });

    it('updates both name and color in one call', () => {
      terminalStore.init();
      const leaf = get(terminalStore).rootPane as TerminalPaneLeaf;
      terminalStore.setPaneLabel(leaf.id, { name: 'build', color: 'coral' });
      const next = get(terminalStore).rootPane as TerminalPaneLeaf;
      expect(next.name).toBe('build');
      expect(next.color).toBe('coral');
    });

    it('clears name when passed null', () => {
      terminalStore.init();
      const leaf = get(terminalStore).rootPane as TerminalPaneLeaf;
      terminalStore.setPaneLabel(leaf.id, { name: 'build', color: 'coral' });
      terminalStore.setPaneLabel(leaf.id, { name: null });
      const next = get(terminalStore).rootPane as TerminalPaneLeaf;
      expect(next.name).toBeUndefined();
      // Color is left alone when its key is absent.
      expect(next.color).toBe('coral');
    });

    it('clears color when passed null', () => {
      terminalStore.init();
      const leaf = get(terminalStore).rootPane as TerminalPaneLeaf;
      terminalStore.setPaneLabel(leaf.id, { name: 'build', color: 'coral' });
      terminalStore.setPaneLabel(leaf.id, { color: null });
      const next = get(terminalStore).rootPane as TerminalPaneLeaf;
      expect(next.color).toBeUndefined();
      expect(next.name).toBe('build');
    });

    it('only touches the matching leaf inside a split', () => {
      terminalStore.init();
      const rootId = (get(terminalStore).rootPane as TerminalPaneLeaf).id;
      const newId = terminalStore.splitPane(rootId, 'vertical', { name: 'right' });
      terminalStore.setPaneLabel(rootId, { name: 'left', color: 'sky' });

      const split = get(terminalStore).rootPane as TerminalPaneSplit;
      const left = split.children.find((c) => c.type === 'terminal' && c.id === rootId) as
        | TerminalPaneLeaf
        | undefined;
      const right = split.children.find((c) => c.type === 'terminal' && c.id === newId) as
        | TerminalPaneLeaf
        | undefined;
      expect(left?.name).toBe('left');
      expect(left?.color).toBe('sky');
      expect(right?.name).toBe('right');
      expect(right?.color).toBeUndefined();
    });

    it('is a no-op when the paneId does not match any leaf', () => {
      terminalStore.init();
      const leaf = get(terminalStore).rootPane as TerminalPaneLeaf;
      terminalStore.setPaneLabel('nonexistent-pane-id', { name: 'x', color: 'rose' });
      const next = get(terminalStore).rootPane as TerminalPaneLeaf;
      expect(next.id).toBe(leaf.id);
      expect(next.name).toBeUndefined();
      expect(next.color).toBeUndefined();
    });

    it('is a no-op when rootPane is null', () => {
      // No init() call — rootPane stays null.
      terminalStore.setPaneLabel('anything', { name: 'x' });
      expect(get(terminalStore).rootPane).toBeNull();
    });
  });

  describe('closePane', () => {
    it('should remove the pane and collapse the split when only one child remains', () => {
      terminalStore.init();
      const rootPaneId = (get(terminalStore).rootPane as { id: string }).id;
      terminalStore.splitPane(rootPaneId, 'horizontal');

      const split = get(terminalStore).rootPane;
      const secondPaneId =
        split?.type === 'split' && split.children[1].type === 'terminal'
          ? split.children[1].id
          : '';

      terminalStore.closePane(secondPaneId);

      const root = get(terminalStore).rootPane;
      expect(root?.type).toBe('terminal');
      if (root?.type === 'terminal') {
        expect(root.id).toBe(rootPaneId);
      }
    });

    it('should set rootPane to null when the last pane is closed', () => {
      terminalStore.init();
      const paneId = (get(terminalStore).rootPane as { id: string }).id;
      terminalStore.closePane(paneId);
      expect(get(terminalStore).rootPane).toBeNull();
    });
  });

  describe('updatePaneSizes', () => {
    it('should update sizes on the matching split', () => {
      terminalStore.init();
      const rootPaneId = (get(terminalStore).rootPane as { id: string }).id;
      terminalStore.splitPane(rootPaneId, 'vertical');

      const splitId = (get(terminalStore).rootPane as { id: string }).id;
      terminalStore.updatePaneSizes(splitId, [25, 75]);

      const root = get(terminalStore).rootPane;
      expect(root?.type).toBe('split');
      if (root?.type === 'split') {
        expect(root.sizes).toEqual([25, 75]);
      }
    });
  });

  describe('reset', () => {
    it('should clear the rootPane', () => {
      terminalStore.init();
      terminalStore.reset();
      expect(get(terminalStore).rootPane).toBeNull();
    });
  });
});

describe('pane tree helpers', () => {
  function buildSampleTree(): TerminalPane {
    return {
      type: 'split',
      id: 'split-1',
      direction: 'vertical',
      children: [
        { type: 'terminal', id: 'pane-a', terminalId: 1 },
        {
          type: 'split',
          id: 'split-2',
          direction: 'horizontal',
          children: [
            { type: 'terminal', id: 'pane-b', terminalId: 2 },
            { type: 'terminal', id: 'pane-c', terminalId: null },
          ],
          sizes: [60, 40],
        },
      ],
      sizes: [50, 50],
    };
  }

  describe('getAllPaneIds', () => {
    it('should return all terminal pane IDs depth-first', () => {
      expect(getAllPaneIds(buildSampleTree())).toEqual(['pane-a', 'pane-b', 'pane-c']);
    });

    it('should return a single id for a leaf', () => {
      expect(getAllPaneIds({ type: 'terminal', id: 'pane-x', terminalId: null })).toEqual([
        'pane-x',
      ]);
    });
  });

  describe('getAllSplitIds', () => {
    it('should return all split IDs depth-first', () => {
      expect(getAllSplitIds(buildSampleTree())).toEqual(['split-1', 'split-2']);
    });

    it('should return an empty array for a leaf', () => {
      expect(getAllSplitIds({ type: 'terminal', id: 'pane-x', terminalId: null })).toEqual([]);
    });
  });

  describe('getFirstTerminalId', () => {
    it('should return the first non-null terminal id depth-first', () => {
      expect(getFirstTerminalId(buildSampleTree())).toBe(1);
    });

    it('should skip panes without a terminalId', () => {
      const tree: TerminalPane = {
        type: 'split',
        id: 'split-1',
        direction: 'vertical',
        children: [
          { type: 'terminal', id: 'a', terminalId: null },
          { type: 'terminal', id: 'b', terminalId: 7 },
        ],
        sizes: [50, 50],
      };
      expect(getFirstTerminalId(tree)).toBe(7);
    });

    it('should return null when no terminal has an id', () => {
      const tree: TerminalPane = { type: 'terminal', id: 'a', terminalId: null };
      expect(getFirstTerminalId(tree)).toBeNull();
    });
  });

  describe('getAllTerminalIds', () => {
    it('should return all non-null terminal ids', () => {
      expect(getAllTerminalIds(buildSampleTree())).toEqual([1, 2]);
    });
  });

  describe('getPaneTerminalIdMap', () => {
    it('should map paneId to terminalId for panes with an id', () => {
      const map = getPaneTerminalIdMap(buildSampleTree());
      expect(Array.from(map.entries())).toEqual([
        ['pane-a', 1],
        ['pane-b', 2],
      ]);
    });
  });

  describe('closePaneInTree', () => {
    it('should remove a leaf', () => {
      const tree: TerminalPane = { type: 'terminal', id: 'a', terminalId: 1 };
      expect(closePaneInTree(tree, 'a')).toBeNull();
    });

    it('should collapse a split when one child remains', () => {
      const tree: TerminalPane = {
        type: 'split',
        id: 'split-1',
        direction: 'vertical',
        children: [
          { type: 'terminal', id: 'a', terminalId: 1 },
          { type: 'terminal', id: 'b', terminalId: 2 },
        ],
        sizes: [50, 50],
      };

      const after = closePaneInTree(tree, 'b');
      expect(after).toEqual({ type: 'terminal', id: 'a', terminalId: 1 });
    });

    it('should rescale sibling sizes when removing one of three', () => {
      const tree: TerminalPane = {
        type: 'split',
        id: 'split-1',
        direction: 'vertical',
        children: [
          { type: 'terminal', id: 'a', terminalId: 1 },
          { type: 'terminal', id: 'b', terminalId: 2 },
          { type: 'terminal', id: 'c', terminalId: 3 },
        ],
        sizes: [20, 30, 50],
      };

      const after = closePaneInTree(tree, 'b');
      expect(after?.type).toBe('split');
      if (after?.type === 'split') {
        // 20 / 70 * 100 ≒ 28.57, 50 / 70 * 100 ≒ 71.43
        expect(after.sizes[0]).toBeCloseTo(28.5714, 3);
        expect(after.sizes[1]).toBeCloseTo(71.4285, 3);
      }
    });
  });
});

describe('getAllLeaves', () => {
  it('returns the single leaf for a terminal pane', () => {
    const leaf: TerminalPaneLeaf = { type: 'terminal', id: 'pane-1', terminalId: 1 };
    expect(getAllLeaves(leaf)).toEqual([leaf]);
  });

  it('flattens leaves depth-first across nested splits', () => {
    const a: TerminalPaneLeaf = { type: 'terminal', id: 'a', terminalId: 1 };
    const b: TerminalPaneLeaf = { type: 'terminal', id: 'b', terminalId: 2 };
    const c: TerminalPaneLeaf = { type: 'terminal', id: 'c', terminalId: 3 };
    const tree: TerminalPaneSplit = {
      type: 'split',
      id: 'split-1',
      direction: 'vertical',
      children: [
        a,
        {
          type: 'split',
          id: 'split-2',
          direction: 'horizontal',
          children: [b, c],
          sizes: [50, 50],
        },
      ],
      sizes: [50, 50],
    };
    expect(getAllLeaves(tree).map((l) => l.id)).toEqual(['a', 'b', 'c']);
  });
});

describe('findPaneLeaf', () => {
  const a: TerminalPaneLeaf = { type: 'terminal', id: 'a', terminalId: 1, name: 'docker' };
  const b: TerminalPaneLeaf = { type: 'terminal', id: 'b', terminalId: 2 };
  const tree: TerminalPaneSplit = {
    type: 'split',
    id: 'split-1',
    direction: 'vertical',
    children: [a, b],
    sizes: [50, 50],
  };

  it('finds a nested leaf by id', () => {
    expect(findPaneLeaf(tree, 'a')).toBe(a);
  });

  it('returns null when the id is a split or missing', () => {
    expect(findPaneLeaf(tree, 'split-1')).toBeNull();
    expect(findPaneLeaf(tree, 'missing')).toBeNull();
  });
});

describe('terminalStore minimized state', () => {
  beforeEach(() => {
    terminalStore.reset();
  });

  /** Build a root pane with `count` sibling panes; returns their ids. */
  function seedPanes(count: number): string[] {
    terminalStore.init();
    const rootId = (get(terminalStore).rootPane as { id: string }).id;
    const ids = [rootId];
    for (let i = 1; i < count; i++) {
      ids.push(terminalStore.splitPane(ids[i - 1], 'vertical'));
    }
    return ids;
  }

  it('isMinimized returns false for unknown panes', () => {
    expect(terminalStore.isMinimized('pane-unknown')).toBe(false);
  });

  it('setMinimized parks a pane while another stays visible', () => {
    const [a, b] = seedPanes(2);
    terminalStore.setMinimized(b, true);
    expect(terminalStore.isMinimized(b)).toBe(true);
    expect(terminalStore.isMinimized(a)).toBe(false);
  });

  it('setMinimized(false) restores a parked pane', () => {
    const [, b] = seedPanes(2);
    terminalStore.setMinimized(b, true);
    terminalStore.setMinimized(b, false);
    expect(terminalStore.isMinimized(b)).toBe(false);
  });

  it('refuses to minimize the last visible pane', () => {
    const [a, b] = seedPanes(2);
    terminalStore.setMinimized(b, true);
    // Only `a` remains visible — minimizing it would empty the layout.
    terminalStore.setMinimized(a, true);
    expect(terminalStore.isMinimized(a)).toBe(false);
  });

  it('refuses to minimize a lone root pane', () => {
    const [a] = seedPanes(1);
    terminalStore.setMinimized(a, true);
    expect(terminalStore.isMinimized(a)).toBe(false);
  });

  it('visiblePaneCount excludes minimized panes', () => {
    const [, b, c] = seedPanes(3);
    expect(terminalStore.visiblePaneCount()).toBe(3);
    terminalStore.setMinimized(b, true);
    expect(terminalStore.visiblePaneCount()).toBe(2);
    terminalStore.setMinimized(c, true);
    expect(terminalStore.visiblePaneCount()).toBe(1);
  });

  it('minimizedLeaves returns parked leaves in tree order', () => {
    const [, b, c] = seedPanes(3);
    terminalStore.setMinimized(c, true);
    terminalStore.setMinimized(b, true);
    expect(terminalStore.minimizedLeaves().map((l) => l.id)).toEqual([b, c]);
  });

  it('getLeaf resolves a pane id to its leaf', () => {
    const [a] = seedPanes(2);
    const leaf = terminalStore.getLeaf(a);
    expect(leaf?.id).toBe(a);
    expect(terminalStore.getLeaf('missing')).toBeNull();
  });

  it('closePane clears the minimized bit for that pane', () => {
    const [, b] = seedPanes(2);
    terminalStore.setMinimized(b, true);
    terminalStore.closePane(b);
    expect(terminalStore.isMinimized(b)).toBe(false);
    expect(terminalStore.minimizedLeaves()).toEqual([]);
  });

  it('closePane un-minimizes the sole survivor to avoid a blank layout', () => {
    const [a, b] = seedPanes(2);
    terminalStore.setMinimized(a, true);
    // Closing the only visible pane (b) would otherwise leave minimized `a`
    // as the root with nothing rendered — `a` must be restored to the layout.
    terminalStore.closePane(b);
    expect(terminalStore.isMinimized(a)).toBe(false);
    expect(terminalStore.visiblePaneCount()).toBe(1);
  });

  it('closePane un-minimizes all survivors when every one was minimized', () => {
    const [a, b, c] = seedPanes(3);
    terminalStore.setMinimized(a, true);
    terminalStore.setMinimized(b, true);
    terminalStore.closePane(c);
    expect(terminalStore.isMinimized(a)).toBe(false);
    expect(terminalStore.isMinimized(b)).toBe(false);
    expect(terminalStore.minimizedLeaves()).toEqual([]);
  });

  it('setMinimized notifies subscribers when state changes', () => {
    const [, b] = seedPanes(2);
    let calls = 0;
    const unsub = terminalStore.subscribe(() => {
      calls++;
    });
    const baseline = calls;
    terminalStore.setMinimized(b, true);
    expect(calls).toBeGreaterThan(baseline);
    unsub();
  });

  it('reset clears all minimized entries', () => {
    const [, b] = seedPanes(2);
    terminalStore.setMinimized(b, true);
    terminalStore.reset();
    expect(terminalStore.isMinimized(b)).toBe(false);
  });
});
