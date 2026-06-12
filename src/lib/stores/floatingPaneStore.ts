import { writable, get } from 'svelte/store';

/**
 * The pane currently shown as a floating "peek" window, or null when none.
 *
 * A minimized pane lives in the footer dock; clicking its chip floats it
 * over the layout as a live, interactive terminal. The float is transient:
 * it returns to the dock as soon as it loses focus, so at most one pane
 * floats at a time. This is pure window-local UI state — it is intentionally
 * not synced across windows (each Tauri webview keeps its own).
 */
function createFloatingPaneStore() {
  const store = writable<string | null>(null);
  const { subscribe, set } = store;

  return {
    subscribe,

    /** Float the given pane (replacing any currently floating one). */
    open: (paneId: string): void => set(paneId),

    /** Dismiss the floating window, returning the pane to the dock. */
    close: (): void => set(null),

    /**
     * Float `paneId`, or dismiss it if it is already the floating pane.
     * Lets a dock chip act as a toggle.
     */
    toggle: (paneId: string): void => {
      set(get(store) === paneId ? null : paneId);
    },

    /** Synchronous read of the currently floating pane id. */
    current: (): string | null => get(store),
  };
}

export const floatingPaneStore = createFloatingPaneStore();
