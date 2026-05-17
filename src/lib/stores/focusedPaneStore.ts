// Backward-compatible facade over `focusedPaneState` (issue #42 phase 1).
//
// Tracks which terminal pane currently has user focus. Used by the CLI
// bridge so `kiri term split` / `term send` etc. with no --pane flag can
// target the pane the user is looking at.
//
// Canonical class: `focusedPaneState.svelte.ts`.

import { writable } from 'svelte/store';
import { focusedPaneState } from './focusedPaneState.svelte';

function createFocusedPaneStore() {
  const mirror = writable<string | null>(focusedPaneState.paneId);
  return {
    subscribe: mirror.subscribe,
    set: (id: string | null) => {
      focusedPaneState.set(id);
      mirror.set(id);
    },
    /** Synchronous read of the current value. */
    current: (): string | null => focusedPaneState.current(),
  };
}

export const focusedPaneStore = createFocusedPaneStore();
