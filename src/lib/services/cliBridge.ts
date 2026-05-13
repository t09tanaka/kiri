import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/**
 * Dependencies the bridge needs from the host App. Passed in (rather
 * than imported) to keep this module unit-testable without Svelte
 * lifecycle.
 */
export interface CliBridgeDeps {
  label: string;
  splitPane: (paneId: string, direction: 'horizontal' | 'vertical') => string;
  closePane: (paneId: string) => void;
  indexOf: (paneId: string) => number;
  resolveFocusedPaneId: () => string | null;
  setPaneCollapsed: (paneId: string, value: boolean) => void;
}

const FOCUSED_SENTINEL = 'focused';

/**
 * Subscribe to `cli:pane-split` / `cli:pane-close` / `cli:pane-minimize`
 * events emitted by the Rust cli_server, dispatch them to the local
 * terminalStore, and reply via the `cli_resolve_pending` Tauri command
 * keyed by `requestId`.
 *
 * Returns a teardown that removes all event listeners. Call it from the
 * caller's cleanup path (typically the App.svelte onMount return).
 */
export async function startCliBridge(deps: CliBridgeDeps): Promise<() => void> {
  const resolveTarget = (paneId: string): string | null => {
    if (paneId === FOCUSED_SENTINEL) return deps.resolveFocusedPaneId();
    return paneId;
  };

  const reply = (requestId: string, payload: Record<string, unknown>): void => {
    void invoke('cli_resolve_pending', {
      label: deps.label,
      requestId,
      payload,
    });
  };

  const unlistenSplit = await listen<{
    requestId: string;
    paneId: string;
    direction: 'horizontal' | 'vertical';
    minimized?: boolean;
  }>('cli:pane-split', (event) => {
    const { requestId, paneId, direction, minimized } = event.payload;
    const target = resolveTarget(paneId);
    if (!target) {
      reply(requestId, { error: 'no_focused_pane' });
      return;
    }
    const newPaneId = deps.splitPane(target, direction);
    if (minimized) deps.setPaneCollapsed(newPaneId, true);
    reply(requestId, { newPaneId, newPaneIndex: deps.indexOf(newPaneId) });
  });

  const unlistenClose = await listen<{ requestId: string; paneId: string }>(
    'cli:pane-close',
    (event) => {
      const { requestId, paneId } = event.payload;
      const target = resolveTarget(paneId);
      if (!target) {
        reply(requestId, { error: 'no_focused_pane' });
        return;
      }
      deps.closePane(target);
      reply(requestId, {});
    }
  );

  const unlistenMinimize = await listen<{
    requestId: string;
    paneId: string;
    minimized: boolean;
  }>('cli:pane-minimize', (event) => {
    const { requestId, paneId, minimized } = event.payload;
    const target = resolveTarget(paneId);
    if (!target) {
      reply(requestId, { error: 'no_focused_pane' });
      return;
    }
    deps.setPaneCollapsed(target, minimized);
    reply(requestId, {});
  });

  return () => {
    unlistenSplit();
    unlistenClose();
    unlistenMinimize();
  };
}
