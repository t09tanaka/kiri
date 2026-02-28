import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  tabStore,
  activeTab,
  getAllPaneIds,
  getAllSplitIds,
  getAllTerminalIds,
  getFirstTerminalId,
  closePaneInTree,
  getPaneTerminalIdMap,
  type TerminalPane,
  type TerminalPaneSplit,
} from './tabStore';

describe('tabStore', () => {
  beforeEach(() => {
    tabStore.reset();
  });

  describe('initial state', () => {
    it('should have empty tabs and null activeTabId', () => {
      const state = get(tabStore);
      expect(state.tabs).toHaveLength(0);
      expect(state.activeTabId).toBeNull();
    });
  });

  describe('addTerminalTab', () => {
    it('should add a terminal tab with default title', () => {
      tabStore.addTerminalTab();

      const state = get(tabStore);
      expect(state.tabs).toHaveLength(1);
      expect(state.tabs[0].type).toBe('terminal');
      expect(state.tabs[0].title).toBe('Terminal');
    });

    it('should use same default title for all tabs', () => {
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();

      const state = get(tabStore);
      expect(state.tabs[0].title).toBe('Terminal');
      expect(state.tabs[1].title).toBe('Terminal');
      expect(state.tabs[2].title).toBe('Terminal');
    });
  });

  describe('closeTab', () => {
    it('should close a tab', () => {
      tabStore.addTerminalTab();
      const tabId = get(tabStore).tabs[0].id;

      tabStore.closeTab(tabId);

      expect(get(tabStore).tabs).toHaveLength(0);
    });

    it('should switch to adjacent tab when closing active tab', () => {
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();

      const state = get(tabStore);
      const middleTabId = state.tabs[1].id;
      tabStore.setActiveTab(middleTabId);

      tabStore.closeTab(middleTabId);

      const newState = get(tabStore);
      expect(newState.tabs).toHaveLength(2);
      // Should switch to the tab at the same index (third tab)
      expect(newState.activeTabId).toBe(newState.tabs[1].id);
    });

    it('should switch to last tab when closing last position active tab', () => {
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();

      const state = get(tabStore);
      const lastTabId = state.tabs[1].id;
      tabStore.setActiveTab(lastTabId);

      tabStore.closeTab(lastTabId);

      const newState = get(tabStore);
      expect(newState.activeTabId).toBe(newState.tabs[0].id);
    });

    it('should set activeTabId to null when closing last tab', () => {
      tabStore.addTerminalTab();
      const tabId = get(tabStore).tabs[0].id;

      tabStore.closeTab(tabId);

      expect(get(tabStore).activeTabId).toBeNull();
    });

    it('should do nothing when closing non-existent tab', () => {
      tabStore.addTerminalTab();

      tabStore.closeTab('non-existent');

      expect(get(tabStore).tabs).toHaveLength(1);
    });

    it('should not change activeTabId when closing non-active tab', () => {
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();

      const state = get(tabStore);
      const firstTabId = state.tabs[0].id;
      const secondTabId = state.tabs[1].id;
      const thirdTabId = state.tabs[2].id;

      // Active tab is the third (last added)
      expect(state.activeTabId).toBe(thirdTabId);

      // Close the first tab (not active)
      tabStore.closeTab(firstTabId);

      const newState = get(tabStore);
      expect(newState.tabs).toHaveLength(2);
      // Active tab should remain the same (third tab)
      expect(newState.activeTabId).toBe(thirdTabId);
      // First tab should be the old second tab
      expect(newState.tabs[0].id).toBe(secondTabId);
    });
  });

  describe('updateTabTitle', () => {
    it('should update tab title', () => {
      tabStore.addTerminalTab();
      const tabId = get(tabStore).tabs[0].id;

      tabStore.updateTabTitle(tabId, 'zsh');

      expect(get(tabStore).tabs[0].title).toBe('zsh');
    });

    it('should only update matching tab', () => {
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();
      const state = get(tabStore);

      tabStore.updateTabTitle(state.tabs[0].id, 'vim');

      const updated = get(tabStore);
      expect(updated.tabs[0].title).toBe('vim');
      expect(updated.tabs[1].title).toBe('Terminal');
    });

    it('should not change anything for non-existent tab', () => {
      tabStore.addTerminalTab();

      tabStore.updateTabTitle('non-existent', 'vim');

      expect(get(tabStore).tabs[0].title).toBe('Terminal');
    });
  });

  describe('setActiveTab', () => {
    it('should set active tab', () => {
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();

      const firstTabId = get(tabStore).tabs[0].id;
      tabStore.setActiveTab(firstTabId);

      expect(get(tabStore).activeTabId).toBe(firstTabId);
    });
  });

  describe('setTerminalId', () => {
    it('should set terminal ID for a pane', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0];
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.setTerminalId(tabId, paneId, 123);

      const updatedTab = get(tabStore).tabs[0];
      expect((updatedTab.rootPane as { terminalId: number }).terminalId).toBe(123);
    });
  });

  describe('splitPane', () => {
    it('should split a pane horizontally', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0];
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'horizontal');

      const updatedTab = get(tabStore).tabs[0];
      expect(updatedTab.rootPane.type).toBe('split');
      if (updatedTab.rootPane.type === 'split') {
        expect(updatedTab.rootPane.direction).toBe('horizontal');
        expect(updatedTab.rootPane.children).toHaveLength(2);
        expect(updatedTab.rootPane.sizes).toEqual([50, 50]);
      }
    });

    it('should split a pane vertically', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0];
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'vertical');

      const updatedTab = get(tabStore).tabs[0];
      expect(updatedTab.rootPane.type).toBe('split');
      if (updatedTab.rootPane.type === 'split') {
        expect(updatedTab.rootPane.direction).toBe('vertical');
      }
    });

    it('should add sibling pane when splitting in same direction (vertical)', () => {
      // Initial state: single pane
      // After first split: [pane-1, pane-2] (50%, 50%)
      // After second split on pane-1: [pane-1, pane-3, pane-2] (33.3%, 33.3%, 33.3%)
      // All panes are distributed equally

      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      const firstPaneId = (state.tabs[0].rootPane as { id: string }).id;

      // First vertical split
      tabStore.splitPane(tabId, firstPaneId, 'vertical');

      state = get(tabStore);
      let rootPane = state.tabs[0].rootPane;
      expect(rootPane.type).toBe('split');
      if (rootPane.type !== 'split') return;
      expect(rootPane.children).toHaveLength(2);

      // Second vertical split on first pane (same direction)
      tabStore.splitPane(tabId, firstPaneId, 'vertical');

      state = get(tabStore);
      rootPane = state.tabs[0].rootPane;
      expect(rootPane.type).toBe('split');
      if (rootPane.type !== 'split') return;

      // Should have 3 children at same level, not nested
      expect(rootPane.children).toHaveLength(3);
      expect(rootPane.direction).toBe('vertical');

      // All children should be terminal panes (no nested splits)
      expect(rootPane.children[0].type).toBe('terminal');
      expect(rootPane.children[1].type).toBe('terminal');
      expect(rootPane.children[2].type).toBe('terminal');

      // All panes distributed equally
      const expectedSize = 100 / 3;
      expect(rootPane.sizes[0]).toBeCloseTo(expectedSize);
      expect(rootPane.sizes[1]).toBeCloseTo(expectedSize);
      expect(rootPane.sizes[2]).toBeCloseTo(expectedSize);
    });

    it('should add sibling pane when splitting in same direction (horizontal)', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      const firstPaneId = (state.tabs[0].rootPane as { id: string }).id;

      // First horizontal split
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      let rootPane = state.tabs[0].rootPane;
      expect(rootPane.type).toBe('split');
      if (rootPane.type !== 'split') return;
      expect(rootPane.children).toHaveLength(2);

      // Second horizontal split on first pane (same direction)
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      rootPane = state.tabs[0].rootPane;
      expect(rootPane.type).toBe('split');
      if (rootPane.type !== 'split') return;

      // Should have 3 children at same level
      expect(rootPane.children).toHaveLength(3);
      expect(rootPane.direction).toBe('horizontal');

      // All panes distributed equally
      const expectedSize = 100 / 3;
      expect(rootPane.sizes[0]).toBeCloseTo(expectedSize);
      expect(rootPane.sizes[1]).toBeCloseTo(expectedSize);
      expect(rootPane.sizes[2]).toBeCloseTo(expectedSize);
    });

    it('should preserve custom sizes when splitting in same direction', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      const firstPaneId = (state.tabs[0].rootPane as { id: string }).id;

      // First vertical split: [pane-1(50%), pane-2(50%)]
      tabStore.splitPane(tabId, firstPaneId, 'vertical');

      state = get(tabStore);
      let rootPane = state.tabs[0].rootPane;
      if (rootPane.type !== 'split') return;

      // User drags boundary to custom sizes: [30%, 70%]
      const pane2Id = (rootPane.children[1] as { id: string }).id;
      tabStore.updatePaneSizes(tabId, rootPane.id, [30, 70]);

      state = get(tabStore);
      rootPane = state.tabs[0].rootPane;
      if (rootPane.type !== 'split') return;
      expect(rootPane.sizes).toEqual([30, 70]);

      // Split pane-2 (70%) in same direction
      tabStore.splitPane(tabId, pane2Id, 'vertical');

      state = get(tabStore);
      rootPane = state.tabs[0].rootPane;
      if (rootPane.type !== 'split') return;

      // All panes distributed equally regardless of previous custom sizes
      expect(rootPane.children).toHaveLength(3);
      const expectedSize = 100 / 3;
      expect(rootPane.sizes[0]).toBeCloseTo(expectedSize);
      expect(rootPane.sizes[1]).toBeCloseTo(expectedSize);
      expect(rootPane.sizes[2]).toBeCloseTo(expectedSize);
    });

    it('should preserve sizes when splitting different direction (nested)', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      const firstPaneId = (state.tabs[0].rootPane as { id: string }).id;

      // First vertical split: [pane-1(50%), pane-2(50%)]
      tabStore.splitPane(tabId, firstPaneId, 'vertical');

      state = get(tabStore);
      let rootPane = state.tabs[0].rootPane;
      if (rootPane.type !== 'split') return;

      const pane2Id = (rootPane.children[1] as { id: string }).id;

      // User drags boundary to custom sizes: [30%, 70%]
      tabStore.updatePaneSizes(tabId, rootPane.id, [30, 70]);

      // Split pane-2 in DIFFERENT direction (horizontal)
      tabStore.splitPane(tabId, pane2Id, 'horizontal');

      state = get(tabStore);
      rootPane = state.tabs[0].rootPane;
      if (rootPane.type !== 'split') return;

      // Outer split sizes should be preserved: [30%, 70%]
      expect(rootPane.sizes).toEqual([30, 70]);
      expect(rootPane.children).toHaveLength(2);

      // Second child should now be a nested horizontal split
      const nestedSplit = rootPane.children[1];
      expect(nestedSplit.type).toBe('split');
      if (nestedSplit.type !== 'split') return;
      expect(nestedSplit.direction).toBe('horizontal');
      expect(nestedSplit.sizes).toEqual([50, 50]);
    });

    it('should create nested split when splitting in different direction', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      const firstPaneId = (state.tabs[0].rootPane as { id: string }).id;

      // First vertical split: [pane-1, pane-2]
      tabStore.splitPane(tabId, firstPaneId, 'vertical');

      state = get(tabStore);
      let rootPane = state.tabs[0].rootPane;
      expect(rootPane.type).toBe('split');
      if (rootPane.type !== 'split') return;

      // Second split on first pane with DIFFERENT direction (horizontal)
      // Should create nested: [[pane-1, pane-3], pane-2]
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      rootPane = state.tabs[0].rootPane;
      expect(rootPane.type).toBe('split');
      if (rootPane.type !== 'split') return;

      // Root should still have 2 children (outer split unchanged)
      expect(rootPane.children).toHaveLength(2);
      expect(rootPane.direction).toBe('vertical');

      // First child should now be a nested split with horizontal direction
      const firstChild = rootPane.children[0];
      expect(firstChild.type).toBe('split');
      if (firstChild.type !== 'split') return;
      expect(firstChild.direction).toBe('horizontal');
      expect(firstChild.children).toHaveLength(2);
    });

    it('should recurse when target is not a direct child in same-direction split', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      const firstPaneId = (state.tabs[0].rootPane as { id: string }).id;

      // Step 1: Vertical split → root: vertical [paneA, paneB]
      tabStore.splitPane(tabId, firstPaneId, 'vertical');
      state = get(tabStore);
      let rootPane = state.tabs[0].rootPane;
      if (rootPane.type !== 'split') return;

      // Step 2: Split paneA horizontally → root: vertical [horizontal[paneA, paneC], paneB]
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');
      state = get(tabStore);
      rootPane = state.tabs[0].rootPane;
      if (rootPane.type !== 'split') return;
      expect(rootPane.direction).toBe('vertical');

      // paneA is now nested inside the horizontal split, NOT a direct child of root
      // Step 3: Split paneA vertically again → root is vertical but paneA is NOT direct child
      // This triggers targetIndex === -1 in same-direction branch, causing recursion
      tabStore.splitPane(tabId, firstPaneId, 'vertical');

      state = get(tabStore);
      rootPane = state.tabs[0].rootPane;
      if (rootPane.type !== 'split') return;

      // Root should still be vertical with 2 children
      expect(rootPane.direction).toBe('vertical');
      expect(rootPane.children).toHaveLength(2);

      // First child should be horizontal split, with one child being a vertical sub-split
      const firstChild = rootPane.children[0];
      expect(firstChild.type).toBe('split');
      if (firstChild.type !== 'split') return;
      expect(firstChild.direction).toBe('horizontal');
    });
  });

  describe('closePane', () => {
    it('should close a pane and flatten the tree', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0];
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'horizontal');

      const splitState = get(tabStore).tabs[0];
      const splitPane = splitState.rootPane as { children: TerminalPane[] };
      const secondPaneId = (splitPane.children[1] as { id: string }).id;

      tabStore.closePane(tabId, secondPaneId);

      const finalTab = get(tabStore).tabs[0];
      expect(finalTab.rootPane.type).toBe('terminal');
    });
  });

  describe('updatePaneSizes', () => {
    it('should update sizes of a split pane', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0];
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'horizontal');

      const splitState = get(tabStore).tabs[0];
      const splitId = (splitState.rootPane as TerminalPaneSplit).id;

      tabStore.updatePaneSizes(tabId, splitId, [30, 70]);

      const updatedTab = get(tabStore).tabs[0];
      expect((updatedTab.rootPane as { sizes: number[] }).sizes).toEqual([30, 70]);
    });
  });

  describe('getActiveTab', () => {
    it('should return active tab', () => {
      tabStore.addTerminalTab();

      const active = tabStore.getActiveTab();

      expect(active).not.toBeNull();
      expect(active?.type).toBe('terminal');
    });

    it('should return null when no tabs', () => {
      const active = tabStore.getActiveTab();

      expect(active).toBeNull();
    });
  });

  describe('reset', () => {
    it('should reset to initial state', () => {
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();

      tabStore.reset();

      const state = get(tabStore);
      expect(state.tabs).toHaveLength(0);
      expect(state.activeTabId).toBeNull();
    });
  });
});

describe('activeTab derived store', () => {
  beforeEach(() => {
    tabStore.reset();
  });

  it('should return null when no tabs', () => {
    expect(get(activeTab)).toBeNull();
  });

  it('should return active tab', () => {
    tabStore.addTerminalTab();

    const active = get(activeTab);
    expect(active).not.toBeNull();
    expect(active?.type).toBe('terminal');
  });
});

describe('getAllPaneIds', () => {
  it('should return single pane ID for leaf', () => {
    const pane: TerminalPane = { type: 'terminal', id: 'pane-1', terminalId: null };

    expect(getAllPaneIds(pane)).toEqual(['pane-1']);
  });

  it('should return all pane IDs for split', () => {
    const pane: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: null },
        { type: 'terminal', id: 'pane-2', terminalId: null },
      ],
      sizes: [50, 50],
    };

    expect(getAllPaneIds(pane)).toEqual(['pane-1', 'pane-2']);
  });

  it('should return all pane IDs for nested splits', () => {
    const pane: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: null },
        {
          type: 'split',
          direction: 'vertical',
          children: [
            { type: 'terminal', id: 'pane-2', terminalId: null },
            { type: 'terminal', id: 'pane-3', terminalId: null },
          ],
          sizes: [50, 50],
        },
      ],
      sizes: [50, 50],
    };

    expect(getAllPaneIds(pane)).toEqual(['pane-1', 'pane-2', 'pane-3']);
  });
});

describe('getAllSplitIds', () => {
  it('should return empty array for leaf', () => {
    const pane: TerminalPane = { type: 'terminal', id: 'pane-1', terminalId: null };

    expect(getAllSplitIds(pane)).toEqual([]);
  });

  it('should return split ID for single split', () => {
    const pane: TerminalPane = {
      type: 'split',
      id: 'split-1',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: null },
        { type: 'terminal', id: 'pane-2', terminalId: null },
      ],
      sizes: [50, 50],
    };

    expect(getAllSplitIds(pane)).toEqual(['split-1']);
  });

  it('should return all split IDs for nested splits', () => {
    const pane: TerminalPane = {
      type: 'split',
      id: 'split-1',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: null },
        {
          type: 'split',
          id: 'split-2',
          direction: 'vertical',
          children: [
            { type: 'terminal', id: 'pane-2', terminalId: null },
            { type: 'terminal', id: 'pane-3', terminalId: null },
          ],
          sizes: [50, 50],
        },
      ],
      sizes: [50, 50],
    };

    expect(getAllSplitIds(pane)).toEqual(['split-1', 'split-2']);
  });
});

describe('getFirstTerminalId', () => {
  it('should return null for leaf with null terminalId', () => {
    const pane: TerminalPane = { type: 'terminal', id: 'pane-1', terminalId: null };

    expect(getFirstTerminalId(pane)).toBeNull();
  });

  it('should return terminal ID for leaf with terminalId', () => {
    const pane: TerminalPane = { type: 'terminal', id: 'pane-1', terminalId: 42 };

    expect(getFirstTerminalId(pane)).toBe(42);
  });

  it('should return first terminal ID in split (depth-first)', () => {
    const pane: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: 10 },
        { type: 'terminal', id: 'pane-2', terminalId: 20 },
      ],
      sizes: [50, 50],
    };

    expect(getFirstTerminalId(pane)).toBe(10);
  });

  it('should skip null terminalIds and return first non-null', () => {
    const pane: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: null },
        { type: 'terminal', id: 'pane-2', terminalId: 20 },
      ],
      sizes: [50, 50],
    };

    expect(getFirstTerminalId(pane)).toBe(20);
  });

  it('should return null when all terminalIds are null', () => {
    const pane: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: null },
        { type: 'terminal', id: 'pane-2', terminalId: null },
      ],
      sizes: [50, 50],
    };

    expect(getFirstTerminalId(pane)).toBeNull();
  });

  it('should traverse nested splits depth-first', () => {
    const pane: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [
        {
          type: 'split',
          direction: 'vertical',
          children: [
            { type: 'terminal', id: 'pane-1', terminalId: null },
            { type: 'terminal', id: 'pane-2', terminalId: 5 },
          ],
          sizes: [50, 50],
        },
        { type: 'terminal', id: 'pane-3', terminalId: 30 },
      ],
      sizes: [50, 50],
    };

    expect(getFirstTerminalId(pane)).toBe(5);
  });
});

describe('getAllTerminalIds', () => {
  it('should return empty array for leaf with null terminalId', () => {
    const pane: TerminalPane = { type: 'terminal', id: 'pane-1', terminalId: null };

    expect(getAllTerminalIds(pane)).toEqual([]);
  });

  it('should return terminal ID for leaf with terminalId', () => {
    const pane: TerminalPane = { type: 'terminal', id: 'pane-1', terminalId: 123 };

    expect(getAllTerminalIds(pane)).toEqual([123]);
  });

  it('should return all terminal IDs for split', () => {
    const pane: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: 1 },
        { type: 'terminal', id: 'pane-2', terminalId: 2 },
      ],
      sizes: [50, 50],
    };

    expect(getAllTerminalIds(pane)).toEqual([1, 2]);
  });
});

describe('advanced tabStore operations', () => {
  beforeEach(() => {
    tabStore.reset();
  });

  describe('complex pane operations', () => {
    it('should handle setTerminalId for non-matching pane', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;

      // Try to set terminal ID for a non-existent pane
      tabStore.setTerminalId(tabId, 'non-existent-pane', 999);

      // Should not throw and pane should remain unchanged
      const updatedTab = get(tabStore).tabs[0];
      expect((updatedTab.rootPane as { terminalId: number | null }).terminalId).toBeNull();
    });

    it('should handle setTerminalId for pane in split (recursive case)', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      let terminalTab = state.tabs[0];
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // Create split with 2 panes
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0];
      const splitPane = terminalTab.rootPane as { children: TerminalPane[] };
      const secondPaneId = (splitPane.children[1] as { id: string }).id;

      // Set terminal ID for the second pane (in the split)
      tabStore.setTerminalId(tabId, secondPaneId, 456);

      const updatedTab = get(tabStore).tabs[0];
      const updatedSplit = updatedTab.rootPane as { children: TerminalPane[] };
      expect((updatedSplit.children[1] as { terminalId: number }).terminalId).toBe(456);
    });

    it('should handle closePane returning null (all children removed)', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0];
      const paneId = (terminalTab.rootPane as { id: string }).id;

      // Close the only pane
      tabStore.closePane(tabId, paneId);

      // Tab should remain (closePane doesn't remove tab when no panes left)
      expect(get(tabStore).tabs).toHaveLength(1);
    });

    it('should handle closing all panes in a split (newChildren.length === 0)', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      let terminalTab = state.tabs[0];
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // Create a nested structure: outer split with pane1 and inner split
      // [pane1, [pane2, pane3]]
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0];
      const outerSplit = terminalTab.rootPane as { children: TerminalPane[] };
      const secondPaneId = (outerSplit.children[1] as { id: string }).id;

      // Split the second pane to create inner split
      tabStore.splitPane(tabId, secondPaneId, 'vertical');

      state = get(tabStore);
      terminalTab = state.tabs[0];
      const outerSplit2 = terminalTab.rootPane as { children: TerminalPane[] };
      const innerSplit = outerSplit2.children[1] as { children: TerminalPane[] };
      const pane2Id = (innerSplit.children[0] as { id: string }).id;
      const pane3Id = (innerSplit.children[1] as { id: string }).id;

      // Close both panes in the inner split - this triggers newChildren.length === 0
      tabStore.closePane(tabId, pane2Id);
      tabStore.closePane(tabId, pane3Id);

      // The inner split should be removed, leaving only pane1
      const finalTab = get(tabStore).tabs[0];
      expect(finalTab.rootPane.type).toBe('terminal');
    });

    it('should handle updatePaneSizes for non-matching first child', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0];
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'horizontal');

      // Try to update sizes with non-matching first child ID
      tabStore.updatePaneSizes(tabId, 'non-existent', [40, 60]);

      // Should not throw and sizes should remain unchanged
      const updatedTab = get(tabStore).tabs[0];
      expect((updatedTab.rootPane as { sizes: number[] }).sizes).toEqual([50, 50]);
    });

    it('should handle nested split with updatePaneSizes', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      let terminalTab = state.tabs[0];
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // First split
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0];
      const splitPane = terminalTab.rootPane as { children: TerminalPane[] };
      const secondPaneId = (splitPane.children[1] as { id: string }).id;

      // Second split (nested)
      tabStore.splitPane(tabId, secondPaneId, 'vertical');

      // Get the nested split's ID
      state = get(tabStore);
      const outerSplit = state.tabs[0].rootPane as TerminalPaneSplit;
      const nestedSplitId = (outerSplit.children[1] as TerminalPaneSplit).id;

      // Update sizes of nested split using split ID
      tabStore.updatePaneSizes(tabId, nestedSplitId, [30, 70]);

      const finalTab = get(tabStore).tabs[0];
      const finalSplit = finalTab.rootPane as { children: TerminalPane[] };
      const nestedSplit = finalSplit.children[1] as { sizes: number[] };
      expect(nestedSplit.sizes).toEqual([30, 70]);
    });

    it('should correctly update nested split sizes without affecting parent', () => {
      // Create a structure where the first child of a split is another split
      // Structure: outerSplit[innerSplit[pane1a, pane1b], pane2]
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      let terminalTab = state.tabs[0];
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // First split: [pane1, pane2]
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0];
      const splitPane = terminalTab.rootPane as TerminalPaneSplit;
      const pane1Id = (splitPane.children[0] as { id: string }).id;

      // Split pane1 to make it a split: outerSplit[innerSplit[pane1a, pane1b], pane2]
      tabStore.splitPane(tabId, pane1Id, 'vertical');

      state = get(tabStore);
      terminalTab = state.tabs[0];
      const outerSplit = terminalTab.rootPane as TerminalPaneSplit;
      const innerSplit = outerSplit.children[0] as TerminalPaneSplit;

      // Update inner split sizes using its own ID
      tabStore.updatePaneSizes(tabId, innerSplit.id, [25, 75]);

      const finalTab = get(tabStore).tabs[0];
      const finalOuterSplit = finalTab.rootPane as TerminalPaneSplit;
      // Inner split sizes should be updated
      expect((finalOuterSplit.children[0] as TerminalPaneSplit).sizes).toEqual([25, 75]);
      // Outer split sizes should NOT be affected
      expect(finalOuterSplit.sizes).toEqual([50, 50]);

      // Now update outer split sizes using its own ID
      tabStore.updatePaneSizes(tabId, outerSplit.id, [40, 60]);

      const finalTab2 = get(tabStore).tabs[0];
      const finalOuterSplit2 = finalTab2.rootPane as TerminalPaneSplit;
      // Outer split sizes should be updated
      expect(finalOuterSplit2.sizes).toEqual([40, 60]);
      // Inner split sizes should NOT be affected
      expect((finalOuterSplit2.children[0] as TerminalPaneSplit).sizes).toEqual([25, 75]);
    });

    it('should close one of multiple children in split', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      let terminalTab = state.tabs[0];
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // Create split with 2 panes
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0];
      const splitPane = terminalTab.rootPane as { children: TerminalPane[] };
      const secondPaneId = (splitPane.children[1] as { id: string }).id;

      // Split second pane to have 3 total
      tabStore.splitPane(tabId, secondPaneId, 'vertical');

      state = get(tabStore);
      terminalTab = state.tabs[0];
      const outerSplit = terminalTab.rootPane as { children: TerminalPane[] };
      const innerSplit = outerSplit.children[1] as { children: TerminalPane[] };
      const thirdPaneId = (innerSplit.children[1] as { id: string }).id;

      // Close one of the nested children
      tabStore.closePane(tabId, thirdPaneId);

      const finalTab = get(tabStore).tabs[0];
      // After closing, the inner split should collapse
      expect(finalTab.rootPane.type).toBe('split');
    });
  });
});

describe('closePaneInTree size preservation', () => {
  it('should redistribute sizes proportionally when closing a pane', () => {
    // 3 panes: [30%, 40%, 30%]
    const pane: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: null },
        { type: 'terminal', id: 'pane-2', terminalId: null },
        { type: 'terminal', id: 'pane-3', terminalId: null },
      ],
      sizes: [30, 40, 30],
    };

    // Close pane-2 (40%)
    const result = closePaneInTree(pane, 'pane-2');

    expect(result).not.toBeNull();
    if (result === null || result.type !== 'split') return;
    expect(result.children).toHaveLength(2);
    // Remaining: pane-1(30) + pane-3(30) = 60 total
    // pane-1: 30/60 * 100 = 50%, pane-3: 30/60 * 100 = 50%
    expect(result.sizes[0]).toBe(50);
    expect(result.sizes[1]).toBe(50);
  });

  it('should redistribute uneven sizes proportionally', () => {
    // 3 panes: [20%, 50%, 30%]
    const pane: TerminalPane = {
      type: 'split',
      direction: 'vertical',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: null },
        { type: 'terminal', id: 'pane-2', terminalId: null },
        { type: 'terminal', id: 'pane-3', terminalId: null },
      ],
      sizes: [20, 50, 30],
    };

    // Close pane-1 (20%)
    const result = closePaneInTree(pane, 'pane-1');

    expect(result).not.toBeNull();
    if (result === null || result.type !== 'split') return;
    expect(result.children).toHaveLength(2);
    // Remaining: pane-2(50) + pane-3(30) = 80 total
    // pane-2: 50/80 * 100 = 62.5%, pane-3: 30/80 * 100 = 37.5%
    expect(result.sizes[0]).toBe(62.5);
    expect(result.sizes[1]).toBe(37.5);
  });
});

describe('closePaneInTree', () => {
  it('should return null when split has single child that is removed (edge case)', () => {
    // This is a defensive edge case - normally splits have 2+ children
    // but the code handles the case where all children are removed
    const splitWithSingleChild: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [{ type: 'terminal', id: 'pane-1', terminalId: null }],
      sizes: [100],
    };

    const result = closePaneInTree(splitWithSingleChild, 'pane-1');

    // When the only child is removed, newChildren.length === 0, returns null
    expect(result).toBeNull();
  });

  it('should return null for nested split where inner split becomes empty', () => {
    // Nested structure where inner split has only one child
    const nestedSplit: TerminalPane = {
      type: 'split',
      direction: 'horizontal',
      children: [
        {
          type: 'split',
          direction: 'vertical',
          children: [{ type: 'terminal', id: 'pane-1', terminalId: null }],
          sizes: [100],
        },
      ],
      sizes: [100],
    };

    const result = closePaneInTree(nestedSplit, 'pane-1');

    // Inner split becomes null (no children), outer split also becomes null
    expect(result).toBeNull();
  });
});

describe('tabStore terminal methods with non-matching tab ID', () => {
  beforeEach(() => {
    tabStore.reset();
  });

  it('setTerminalId should not affect other tabs', () => {
    // Add two terminal tabs
    tabStore.addTerminalTab();
    tabStore.addTerminalTab();

    const state = get(tabStore);
    const tab1Id = state.tabs[0].id;
    const tab2Pane = (state.tabs[1].rootPane as { id: string }).id;

    // Set terminal ID on first tab, but reference second tab's pane ID
    // This tests the branch where t.id !== tabId
    tabStore.setTerminalId(tab1Id, tab2Pane, 123);

    // Second tab should be unchanged (its pane ID won't match in first tab's tree)
    const updatedState = get(tabStore);
    expect((updatedState.tabs[1].rootPane as { terminalId: number | null }).terminalId).toBeNull();
  });

  it('setTerminalId should not affect tabs with different IDs', () => {
    tabStore.addTerminalTab();
    tabStore.addTerminalTab();

    const state = get(tabStore);
    const firstTabPane = (state.tabs[0].rootPane as { id: string }).id;

    // Set terminal ID targeting second tab ID but it doesn't exist
    tabStore.setTerminalId('non-existent-tab', firstTabPane, 456);

    // Both tabs should be unchanged
    const updatedState = get(tabStore);
    expect((updatedState.tabs[0].rootPane as { terminalId: number | null }).terminalId).toBeNull();
    expect((updatedState.tabs[1].rootPane as { terminalId: number | null }).terminalId).toBeNull();
  });

  it('splitPane should not affect other tabs', () => {
    tabStore.addTerminalTab();
    tabStore.addTerminalTab();

    const state = get(tabStore);
    const tab2Pane = (state.tabs[1].rootPane as { id: string }).id;

    // Split targeting non-existent tab
    tabStore.splitPane('non-existent-tab', tab2Pane, 'horizontal');

    // Both tabs should remain unchanged (no split)
    const updatedState = get(tabStore);
    expect(updatedState.tabs[0].rootPane.type).toBe('terminal');
    expect(updatedState.tabs[1].rootPane.type).toBe('terminal');
  });

  it('closePane should not affect other tabs', () => {
    tabStore.addTerminalTab();
    tabStore.addTerminalTab();

    const state = get(tabStore);
    const tab2Pane = (state.tabs[1].rootPane as { id: string }).id;

    // Close targeting non-existent tab
    tabStore.closePane('non-existent-tab', tab2Pane);

    // Both tabs should remain unchanged
    const updatedState = get(tabStore);
    expect(updatedState.tabs).toHaveLength(2);
  });

  it('updatePaneSizes should not affect other tabs', () => {
    // Create a tab with a split
    tabStore.addTerminalTab();
    tabStore.addTerminalTab();

    let state = get(tabStore);
    const tab1Id = state.tabs[0].id;
    const pane1Id = (state.tabs[0].rootPane as { id: string }).id;

    // Split first tab
    tabStore.splitPane(tab1Id, pane1Id, 'horizontal');

    state = get(tabStore);
    const splitPane = state.tabs[0].rootPane as { children: TerminalPane[]; sizes: number[] };
    const firstChildId = (splitPane.children[0] as { id: string }).id;

    // Update sizes targeting non-existent tab
    tabStore.updatePaneSizes('non-existent-tab', firstChildId, [30, 70]);

    // First tab's sizes should be unchanged
    const updatedState = get(tabStore);
    expect((updatedState.tabs[0].rootPane as { sizes: number[] }).sizes).toEqual([50, 50]);
  });
});

describe('getPaneTerminalIdMap', () => {
  it('should return map with single entry for leaf pane with terminalId', () => {
    const pane: TerminalPane = { type: 'terminal', id: 'pane-1', terminalId: 42 };
    const map = getPaneTerminalIdMap(pane);

    expect(map.size).toBe(1);
    expect(map.get('pane-1')).toBe(42);
  });

  it('should return empty map for leaf pane with null terminalId', () => {
    const pane: TerminalPane = { type: 'terminal', id: 'pane-1', terminalId: null };
    const map = getPaneTerminalIdMap(pane);

    expect(map.size).toBe(0);
  });

  it('should return all pane-terminalId pairs for split pane tree', () => {
    const pane: TerminalPane = {
      type: 'split',
      id: 'split-1',
      direction: 'horizontal',
      children: [
        { type: 'terminal', id: 'pane-1', terminalId: 10 },
        {
          type: 'split',
          id: 'split-2',
          direction: 'vertical',
          children: [
            { type: 'terminal', id: 'pane-2', terminalId: 20 },
            { type: 'terminal', id: 'pane-3', terminalId: null },
          ],
          sizes: [50, 50],
        },
      ],
      sizes: [50, 50],
    };
    const map = getPaneTerminalIdMap(pane);

    expect(map.size).toBe(2);
    expect(map.get('pane-1')).toBe(10);
    expect(map.get('pane-2')).toBe(20);
    expect(map.has('pane-3')).toBe(false);
  });
});
