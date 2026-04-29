import { writable } from 'svelte/store';

/**
 * Tracks which terminal pane currently has user focus.
 *
 * Used by the CLI bridge so `kiri term split` / `term send` etc. with no
 * --pane flag can target the pane the user is looking at.
 */
function createFocusedPaneStore() {
  const { subscribe, set } = writable<string | null>(null);
  let value: string | null = null;
  subscribe((v) => {
    value = v;
  });
  return {
    subscribe,
    set,
    /** Synchronous read of the current value. */
    current: (): string | null => value,
  };
}

export const focusedPaneStore = createFocusedPaneStore();
