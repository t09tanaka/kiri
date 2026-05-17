// Backward-compatible facade over `peekState` (issue #42 phase 1).
// Canonical class: `peekState.svelte.ts`.

import { writable, derived } from 'svelte/store';
import { peekState, type PeekStateShape } from './peekState.svelte';

export type PeekState = PeekStateShape;

function snapshot(): PeekStateShape {
  return { ...peekState.state };
}

function createPeekStore() {
  const mirror = writable<PeekStateShape>(snapshot());
  const refresh = () => mirror.set(snapshot());

  return {
    subscribe: mirror.subscribe,

    open: (filePath: string, lineNumber?: number, columnNumber?: number) => {
      peekState.open(filePath, lineNumber, columnNumber);
      refresh();
    },

    close: () => {
      peekState.close();
      refresh();
    },
  };
}

export const peekStore = createPeekStore();

export const isPeekOpen = derived(peekStore, ($peek) => $peek.isOpen);
