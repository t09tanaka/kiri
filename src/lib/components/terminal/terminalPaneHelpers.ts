import { getAllPaneIds, terminalStore, type TerminalPane } from '@/lib/stores/terminalStore';

/**
 * Whether the given paneId still exists somewhere in the terminal store
 * tree. Used by Terminal.svelte's onDestroy hook to decide between a
 * "real close" (dispose) and a "remount during split" (preserve).
 */
export function paneExistsInStore(paneId: string): boolean {
  const state = terminalStore.getState();
  if (!state.rootPane) return false;
  return getAllPaneIds(state.rootPane).includes(paneId);
}

/**
 * Walk the pane tree and return the PTY id assigned to `paneId`, or
 * `null` if the pane has none yet (newly created). When non-null we
 * reattach the existing PTY instead of creating a new one.
 */
export function getExistingTerminalId(paneId: string): number | null {
  const state = terminalStore.getState();
  if (!state.rootPane) return null;

  const findTerminalId = (pane: TerminalPane): number | null => {
    if (pane.type === 'terminal') {
      return pane.id === paneId ? pane.terminalId : null;
    }
    for (const child of pane.children) {
      const result = findTerminalId(child);
      if (result !== null) return result;
    }
    return null;
  };
  return findTerminalId(state.rootPane);
}
