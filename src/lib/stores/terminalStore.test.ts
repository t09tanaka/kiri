import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  terminalStore,
  closePaneInTree,
  getAllPaneIds,
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
