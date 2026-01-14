import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  tabStore,
  activeTab,
  getAllPaneIds,
  getAllTerminalIds,
  closePaneInTree,
  type TerminalPane,
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

  describe('addEditorTab', () => {
    it('should add an editor tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');

      const state = get(tabStore);
      expect(state.tabs).toHaveLength(1);
      expect(state.tabs[0].type).toBe('editor');
      expect((state.tabs[0] as { filePath: string }).filePath).toBe('/path/to/file.ts');
      expect(state.activeTabId).toBe(state.tabs[0].id);
    });

    it('should not duplicate editor tab for same file', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      tabStore.addEditorTab('/path/to/file.ts');

      const state = get(tabStore);
      expect(state.tabs).toHaveLength(1);
    });

    it('should switch to existing tab when adding same file', () => {
      tabStore.addEditorTab('/path/to/file1.ts');
      tabStore.addEditorTab('/path/to/file2.ts');
      const firstTabId = get(tabStore).tabs[0].id;

      tabStore.addEditorTab('/path/to/file1.ts');

      expect(get(tabStore).activeTabId).toBe(firstTabId);
    });
  });

  describe('addTerminalTab', () => {
    it('should add a terminal tab', () => {
      tabStore.addTerminalTab();

      const state = get(tabStore);
      expect(state.tabs).toHaveLength(1);
      expect(state.tabs[0].type).toBe('terminal');
      expect((state.tabs[0] as { title: string }).title).toBe('Terminal 1');
    });

    it('should increment terminal count in title', () => {
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();
      tabStore.addTerminalTab();

      const state = get(tabStore);
      expect((state.tabs[0] as { title: string }).title).toBe('Terminal 1');
      expect((state.tabs[1] as { title: string }).title).toBe('Terminal 2');
      expect((state.tabs[2] as { title: string }).title).toBe('Terminal 3');
    });
  });

  describe('closeTab', () => {
    it('should close a tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      const tabId = get(tabStore).tabs[0].id;

      tabStore.closeTab(tabId);

      expect(get(tabStore).tabs).toHaveLength(0);
    });

    it('should switch to adjacent tab when closing active tab', () => {
      tabStore.addEditorTab('/path/to/file1.ts');
      tabStore.addEditorTab('/path/to/file2.ts');
      tabStore.addEditorTab('/path/to/file3.ts');

      const state = get(tabStore);
      const middleTabId = state.tabs[1].id;
      tabStore.setActiveTab(middleTabId);

      tabStore.closeTab(middleTabId);

      const newState = get(tabStore);
      expect(newState.tabs).toHaveLength(2);
      // Should switch to the tab at the same index (file3)
      expect(newState.activeTabId).toBe(newState.tabs[1].id);
    });

    it('should switch to last tab when closing last position active tab', () => {
      tabStore.addEditorTab('/path/to/file1.ts');
      tabStore.addEditorTab('/path/to/file2.ts');

      const state = get(tabStore);
      const lastTabId = state.tabs[1].id;
      tabStore.setActiveTab(lastTabId);

      tabStore.closeTab(lastTabId);

      const newState = get(tabStore);
      expect(newState.activeTabId).toBe(newState.tabs[0].id);
    });

    it('should set activeTabId to null when closing last tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      const tabId = get(tabStore).tabs[0].id;

      tabStore.closeTab(tabId);

      expect(get(tabStore).activeTabId).toBeNull();
    });

    it('should do nothing when closing non-existent tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');

      tabStore.closeTab('non-existent');

      expect(get(tabStore).tabs).toHaveLength(1);
    });

    it('should not change activeTabId when closing non-active tab', () => {
      tabStore.addEditorTab('/path/to/file1.ts');
      tabStore.addEditorTab('/path/to/file2.ts');
      tabStore.addEditorTab('/path/to/file3.ts');

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

  describe('setActiveTab', () => {
    it('should set active tab', () => {
      tabStore.addEditorTab('/path/to/file1.ts');
      tabStore.addEditorTab('/path/to/file2.ts');

      const firstTabId = get(tabStore).tabs[0].id;
      tabStore.setActiveTab(firstTabId);

      expect(get(tabStore).activeTabId).toBe(firstTabId);
    });
  });

  describe('setModified', () => {
    it('should set modified flag for editor tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      const tabId = get(tabStore).tabs[0].id;

      tabStore.setModified(tabId, true);

      const tab = get(tabStore).tabs[0];
      expect((tab as { modified: boolean }).modified).toBe(true);
    });

    it('should not affect terminal tabs', () => {
      tabStore.addTerminalTab();
      const tabId = get(tabStore).tabs[0].id;

      tabStore.setModified(tabId, true);

      const tab = get(tabStore).tabs[0];
      expect(tab.type).toBe('terminal');
    });
  });

  describe('setExternallyModified', () => {
    it('should set externallyModified flag for editor tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      const tabId = get(tabStore).tabs[0].id;

      tabStore.setExternallyModified(tabId, true);

      const tab = get(tabStore).tabs[0];
      expect((tab as { externallyModified: boolean }).externallyModified).toBe(true);
    });

    it('should not affect terminal tabs', () => {
      tabStore.addTerminalTab();
      const tabId = get(tabStore).tabs[0].id;

      tabStore.setExternallyModified(tabId, true);

      const tab = get(tabStore).tabs[0];
      expect(tab.type).toBe('terminal');
    });

    it('should clear externallyModified flag', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      const tabId = get(tabStore).tabs[0].id;

      tabStore.setExternallyModified(tabId, true);
      tabStore.setExternallyModified(tabId, false);

      const tab = get(tabStore).tabs[0];
      expect((tab as { externallyModified: boolean }).externallyModified).toBe(false);
    });
  });

  describe('setExternallyModifiedByPath', () => {
    it('should set externallyModified flag by file path', () => {
      tabStore.addEditorTab('/path/to/file.ts');

      tabStore.setExternallyModifiedByPath('/path/to/file.ts', true);

      const tab = get(tabStore).tabs[0];
      expect((tab as { externallyModified: boolean }).externallyModified).toBe(true);
    });

    it('should not affect other editor tabs', () => {
      tabStore.addEditorTab('/path/to/file1.ts');
      tabStore.addEditorTab('/path/to/file2.ts');

      tabStore.setExternallyModifiedByPath('/path/to/file1.ts', true);

      const tab1 = get(tabStore).tabs[0];
      const tab2 = get(tabStore).tabs[1];
      expect((tab1 as { externallyModified: boolean }).externallyModified).toBe(true);
      expect((tab2 as { externallyModified: boolean }).externallyModified).toBe(false);
    });

    it('should do nothing for non-matching path', () => {
      tabStore.addEditorTab('/path/to/file.ts');

      tabStore.setExternallyModifiedByPath('/path/to/other.ts', true);

      const tab = get(tabStore).tabs[0];
      expect((tab as { externallyModified: boolean }).externallyModified).toBe(false);
    });
  });

  describe('setTerminalId', () => {
    it('should set terminal ID for a pane', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.setTerminalId(tabId, paneId, 123);

      const updatedTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      expect((updatedTab.rootPane as { terminalId: number }).terminalId).toBe(123);
    });
  });

  describe('splitPane', () => {
    it('should split a pane horizontally', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'horizontal');

      const updatedTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
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
      const terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'vertical');

      const updatedTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      expect(updatedTab.rootPane.type).toBe('split');
      if (updatedTab.rootPane.type === 'split') {
        expect(updatedTab.rootPane.direction).toBe('vertical');
      }
    });
  });

  describe('closePane', () => {
    it('should close a pane and flatten the tree', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'horizontal');

      const splitState = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      const splitPane = splitState.rootPane as { children: TerminalPane[] };
      const secondPaneId = (splitPane.children[1] as { id: string }).id;

      tabStore.closePane(tabId, secondPaneId);

      const finalTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      expect(finalTab.rootPane.type).toBe('terminal');
    });
  });

  describe('updatePaneSizes', () => {
    it('should update sizes of a split pane', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'horizontal');

      const splitState = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      const firstChildId = (
        (splitState.rootPane as { children: TerminalPane[] }).children[0] as { id: string }
      ).id;

      tabStore.updatePaneSizes(tabId, firstChildId, [30, 70]);

      const updatedTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      expect((updatedTab.rootPane as { sizes: number[] }).sizes).toEqual([30, 70]);
    });
  });

  describe('getActiveTab', () => {
    it('should return active tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');

      const active = tabStore.getActiveTab();

      expect(active).not.toBeNull();
      expect(active?.type).toBe('editor');
    });

    it('should return null when no tabs', () => {
      const active = tabStore.getActiveTab();

      expect(active).toBeNull();
    });
  });

  describe('getStateForPersistence', () => {
    it('should convert tabs to persisted format', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      tabStore.addTerminalTab();

      const { tabs, activeTabId } = tabStore.getStateForPersistence();

      expect(tabs).toHaveLength(2);
      expect(tabs[0].type).toBe('editor');
      expect(tabs[0].filePath).toBe('/path/to/file.ts');
      expect(tabs[1].type).toBe('terminal');
      expect(tabs[1].title).toBe('Terminal 1');
      expect(activeTabId).not.toBeNull();
    });
  });

  describe('restoreState', () => {
    it('should restore tabs from persisted format', () => {
      const persistedTabs = [
        { id: 'tab-1', type: 'editor' as const, filePath: '/path/to/file.ts' },
        { id: 'tab-2', type: 'terminal' as const, title: 'My Terminal' },
      ];

      tabStore.restoreState(persistedTabs, 'tab-1');

      const state = get(tabStore);
      expect(state.tabs).toHaveLength(2);
      expect(state.activeTabId).toBe('tab-1');
    });

    it('should set activeTabId to null if not found', () => {
      const persistedTabs = [
        { id: 'tab-1', type: 'editor' as const, filePath: '/path/to/file.ts' },
      ];

      tabStore.restoreState(persistedTabs, 'non-existent');

      expect(get(tabStore).activeTabId).toBeNull();
    });

    it('should handle editor tabs without filePath', () => {
      const persistedTabs = [{ id: 'tab-1', type: 'editor' as const, filePath: undefined }];

      tabStore.restoreState(persistedTabs, null);

      expect(get(tabStore).tabs).toHaveLength(0);
    });
  });

  describe('reset', () => {
    it('should reset to initial state', () => {
      tabStore.addEditorTab('/path/to/file.ts');
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
    tabStore.addEditorTab('/path/to/file.ts');

    const active = get(activeTab);
    expect(active).not.toBeNull();
    expect(active?.type).toBe('editor');
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
      const updatedTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      expect((updatedTab.rootPane as { terminalId: number | null }).terminalId).toBeNull();
    });

    it('should handle setTerminalId for pane in split (recursive case)', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      let terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // Create split with 2 panes
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const splitPane = terminalTab.rootPane as { children: TerminalPane[] };
      const secondPaneId = (splitPane.children[1] as { id: string }).id;

      // Set terminal ID for the second pane (in the split)
      tabStore.setTerminalId(tabId, secondPaneId, 456);

      const updatedTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      const updatedSplit = updatedTab.rootPane as { children: TerminalPane[] };
      expect((updatedSplit.children[1] as { terminalId: number }).terminalId).toBe(456);
    });

    it('should handle splitPane for non-terminal tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      const tabId = get(tabStore).tabs[0].id;

      // Try to split pane on editor tab (should do nothing)
      tabStore.splitPane(tabId, 'some-pane', 'horizontal');

      // Should not throw
      expect(get(tabStore).tabs).toHaveLength(1);
    });

    it('should handle setTerminalId for non-terminal tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      const tabId = get(tabStore).tabs[0].id;

      // Try to set terminal ID on editor tab (should do nothing)
      tabStore.setTerminalId(tabId, 'some-pane', 999);

      // Should not throw and tab should remain an editor
      const tab = get(tabStore).tabs[0];
      expect(tab.type).toBe('editor');
    });

    it('should handle closePane for non-terminal tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      const tabId = get(tabStore).tabs[0].id;

      // Try to close pane on editor tab (should do nothing)
      tabStore.closePane(tabId, 'some-pane');

      // Should not throw
      expect(get(tabStore).tabs).toHaveLength(1);
    });

    it('should handle updatePaneSizes for non-terminal tab', () => {
      tabStore.addEditorTab('/path/to/file.ts');
      const tabId = get(tabStore).tabs[0].id;

      // Try to update pane sizes on editor tab (should do nothing)
      tabStore.updatePaneSizes(tabId, 'some-pane', [40, 60]);

      // Should not throw
      expect(get(tabStore).tabs).toHaveLength(1);
    });

    it('should handle closePane returning null (all children removed)', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0] as { rootPane: TerminalPane };
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
      let terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // Create a nested structure: outer split with pane1 and inner split
      // [pane1, [pane2, pane3]]
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const outerSplit = terminalTab.rootPane as { children: TerminalPane[] };
      const secondPaneId = (outerSplit.children[1] as { id: string }).id;

      // Split the second pane to create inner split
      tabStore.splitPane(tabId, secondPaneId, 'vertical');

      state = get(tabStore);
      terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const outerSplit2 = terminalTab.rootPane as { children: TerminalPane[] };
      const innerSplit = outerSplit2.children[1] as { children: TerminalPane[] };
      const pane2Id = (innerSplit.children[0] as { id: string }).id;
      const pane3Id = (innerSplit.children[1] as { id: string }).id;

      // Close both panes in the inner split - this triggers newChildren.length === 0
      tabStore.closePane(tabId, pane2Id);
      tabStore.closePane(tabId, pane3Id);

      // The inner split should be removed, leaving only pane1
      const finalTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      expect(finalTab.rootPane.type).toBe('terminal');
    });

    it('should handle updatePaneSizes for non-matching first child', () => {
      tabStore.addTerminalTab();
      const state = get(tabStore);
      const tabId = state.tabs[0].id;
      const terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const paneId = (terminalTab.rootPane as { id: string }).id;

      tabStore.splitPane(tabId, paneId, 'horizontal');

      // Try to update sizes with non-matching first child ID
      tabStore.updatePaneSizes(tabId, 'non-existent', [40, 60]);

      // Should not throw and sizes should remain unchanged
      const updatedTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      expect((updatedTab.rootPane as { sizes: number[] }).sizes).toEqual([50, 50]);
    });

    it('should handle nested split with updatePaneSizes', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      let terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // First split
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const splitPane = terminalTab.rootPane as { children: TerminalPane[] };
      const secondPaneId = (splitPane.children[1] as { id: string }).id;

      // Second split (nested)
      tabStore.splitPane(tabId, secondPaneId, 'vertical');

      // Update sizes of nested split
      tabStore.updatePaneSizes(tabId, secondPaneId, [30, 70]);

      const finalTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      const finalSplit = finalTab.rootPane as { children: TerminalPane[] };
      const nestedSplit = finalSplit.children[1] as { sizes: number[] };
      expect(nestedSplit.sizes).toEqual([30, 70]);
    });

    it('should handle updatePaneSizes when first child is a split (not terminal)', () => {
      // Create a structure where the first child of a split is another split
      // Structure: split[split[pane1a, pane1b], pane2]
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      let terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // First split: [pane1, pane2]
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const splitPane = terminalTab.rootPane as { children: TerminalPane[] };
      const pane1Id = (splitPane.children[0] as { id: string }).id;

      // Split pane1 to make it a split: split[split[pane1a, pane1b], pane2]
      tabStore.splitPane(tabId, pane1Id, 'vertical');

      state = get(tabStore);
      terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const outerSplit = terminalTab.rootPane as { children: TerminalPane[] };
      // First child is now a split (innerSplit)
      const innerSplit = outerSplit.children[0] as { children: TerminalPane[] };
      const innerFirstPaneId = (innerSplit.children[0] as { id: string }).id;

      // updatePaneSizes identifies a split by its first child's ID
      // When first child is a split, it uses getAllPaneIds(firstChild)[0]
      // So using innerFirstPaneId identifies the OUTER split (whose first child is innerSplit)
      // This tests the branch where firstChild.type !== 'terminal'
      tabStore.updatePaneSizes(tabId, innerFirstPaneId, [25, 75]);

      // The OUTER split's sizes should be updated
      const finalTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      const finalOuterSplit = finalTab.rootPane as { sizes: number[] };
      expect(finalOuterSplit.sizes).toEqual([25, 75]);
    });

    it('should close one of multiple children in split', () => {
      tabStore.addTerminalTab();
      let state = get(tabStore);
      const tabId = state.tabs[0].id;
      let terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const firstPaneId = (terminalTab.rootPane as { id: string }).id;

      // Create split with 2 panes
      tabStore.splitPane(tabId, firstPaneId, 'horizontal');

      state = get(tabStore);
      terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const splitPane = terminalTab.rootPane as { children: TerminalPane[] };
      const secondPaneId = (splitPane.children[1] as { id: string }).id;

      // Split second pane to have 3 total
      tabStore.splitPane(tabId, secondPaneId, 'vertical');

      state = get(tabStore);
      terminalTab = state.tabs[0] as { rootPane: TerminalPane };
      const outerSplit = terminalTab.rootPane as { children: TerminalPane[] };
      const innerSplit = outerSplit.children[1] as { children: TerminalPane[] };
      const thirdPaneId = (innerSplit.children[1] as { id: string }).id;

      // Close one of the nested children
      tabStore.closePane(tabId, thirdPaneId);

      const finalTab = get(tabStore).tabs[0] as { rootPane: TerminalPane };
      // After closing, the inner split should collapse
      expect(finalTab.rootPane.type).toBe('split');
    });
  });

  describe('restoreState edge cases', () => {
    it('should restore terminal tab without title', () => {
      const persistedTabs = [{ id: 'tab-1', type: 'terminal' as const }];

      tabStore.restoreState(persistedTabs, 'tab-1');

      const state = get(tabStore);
      expect(state.tabs).toHaveLength(1);
      const tab = state.tabs[0] as { title: string };
      expect(tab.title).toBe('Terminal 1');
    });

    it('should update nextId based on existing tab IDs', () => {
      const persistedTabs = [{ id: 'tab-100', type: 'editor' as const, filePath: '/path/file.ts' }];

      tabStore.restoreState(persistedTabs, 'tab-100');
      tabStore.addEditorTab('/new/file.ts');

      const state = get(tabStore);
      // New tab should have ID > 100
      const newTabId = state.tabs[1].id;
      const numericPart = parseInt(newTabId.replace('tab-', ''), 10);
      expect(numericPart).toBeGreaterThan(100);
    });

    it('should handle non-standard tab ID format (fallback to 0)', () => {
      // Tab ID doesn't match tab-{number} pattern
      const persistedTabs = [
        { id: 'custom-id', type: 'editor' as const, filePath: '/path/file.ts' },
        { id: 'another-custom', type: 'editor' as const, filePath: '/path/file2.ts' },
      ];

      tabStore.restoreState(persistedTabs, 'custom-id');
      tabStore.addEditorTab('/new/file.ts');

      const state = get(tabStore);
      // New tab should have ID starting from 1 (since non-standard IDs return 0)
      const newTabId = state.tabs[2].id;
      expect(newTabId).toMatch(/^tab-\d+$/);
    });

    it('should handle non-standard pane ID format in terminal tabs (fallback to 0)', () => {
      // Pane ID doesn't match pane-{number} pattern
      const persistedTabs = [{ id: 'tab-1', type: 'terminal' as const, title: 'Terminal' }];

      tabStore.restoreState(persistedTabs, 'tab-1');

      // The restored terminal tab will have a newly generated rootPane
      // Add a new terminal tab to verify pane ID generation works
      tabStore.addTerminalTab();

      const state = get(tabStore);
      expect(state.tabs).toHaveLength(2);
      const newTerminalTab = state.tabs[1] as { rootPane: TerminalPane };
      expect((newTerminalTab.rootPane as { id: string }).id).toMatch(/^pane-\d+$/);
    });
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
