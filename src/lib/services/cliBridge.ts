import { invoke } from '@tauri-apps/api/core';
import { eventService } from '@/lib/services/eventService';
import type { PaneColor } from '@/lib/stores/terminalStore';

/**
 * Dependencies the bridge needs from the host App. Passed in (rather
 * than imported) to keep this module unit-testable without Svelte
 * lifecycle.
 */
export interface CliBridgeDeps {
  label: string;
  splitPane: (
    paneId: string,
    direction: 'horizontal' | 'vertical',
    opts?: { name?: string; color?: PaneColor }
  ) => string;
  closePane: (paneId: string) => void;
  indexOf: (paneId: string) => number;
  resolveFocusedPaneId: () => string | null;
  setPaneCollapsed: (paneId: string, value: boolean) => void;
  /**
   * Update an existing pane's label.
   *
   * `name`/`color` use a three-state convention: `undefined` leaves the
   * field alone, `null` clears it, a value installs it. The bridge
   * builds this from the wire's separate `setName`/`clearName`
   * (and color) fields so the store stays self-contained.
   */
  setPaneLabel: (paneId: string, opts: { name?: string | null; color?: PaneColor | null }) => void;
}

const FOCUSED_SENTINEL = 'focused';

/**
 * Subscribe to `cli:pane-split` / `cli:pane-close` / `cli:pane-minimize`
 * / `cli:pane-set-label` events emitted by the Rust cli_server, dispatch
 * them to the local terminalStore, and reply via the `cli_resolve_pending`
 * Tauri command keyed by `requestId`.
 *
 * Each listener is scoped to the current window (via
 * `eventService.listenCurrentWindow`) so events that the Rust side targets
 * with `emit_to(label, ...)` do not leak into other open windows. See
 * `.claude/rules/multi-window.md`.
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

  const unlistenSplit = await eventService.listenCurrentWindow<{
    requestId: string;
    paneId: string;
    direction: 'horizontal' | 'vertical';
    name?: string;
    color?: PaneColor;
    minimized?: boolean;
  }>('cli:pane-split', (event) => {
    const { requestId, paneId, direction, name, color, minimized } = event.payload;
    const target = resolveTarget(paneId);
    if (!target) {
      reply(requestId, { error: 'no_focused_pane' });
      return;
    }
    const newPaneId = deps.splitPane(target, direction, { name, color });
    if (minimized) deps.setPaneCollapsed(newPaneId, true);
    reply(requestId, { newPaneId, newPaneIndex: deps.indexOf(newPaneId) });
  });

  const unlistenClose = await eventService.listenCurrentWindow<{
    requestId: string;
    paneId: string;
  }>('cli:pane-close', (event) => {
    const { requestId, paneId } = event.payload;
    const target = resolveTarget(paneId);
    if (!target) {
      reply(requestId, { error: 'no_focused_pane' });
      return;
    }
    deps.closePane(target);
    reply(requestId, {});
  });

  const unlistenMinimize = await eventService.listenCurrentWindow<{
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

  const unlistenSetLabel = await eventService.listenCurrentWindow<{
    requestId: string;
    paneId: string;
    setName?: string | null;
    clearName?: boolean;
    setColor?: PaneColor | null;
    clearColor?: boolean;
  }>('cli:pane-set-label', (event) => {
    const { requestId, paneId, setName, clearName, setColor, clearColor } = event.payload;
    const target = resolveTarget(paneId);
    if (!target) {
      reply(requestId, { error: 'no_focused_pane' });
      return;
    }
    const opts: { name?: string | null; color?: PaneColor | null } = {};
    if (clearName) opts.name = null;
    else if (typeof setName === 'string') opts.name = setName;
    if (clearColor) opts.color = null;
    else if (setColor !== undefined && setColor !== null) opts.color = setColor;
    deps.setPaneLabel(target, opts);
    reply(requestId, {});
  });

  return () => {
    unlistenSplit();
    unlistenClose();
    unlistenMinimize();
    unlistenSetLabel();
  };
}
