// Backward-compatible facade over `diffViewState` (issue #42 phase 1).
// See the canonical class in `diffViewState.svelte.ts`.

import { writable, derived } from 'svelte/store';
import { diffViewState, type DiffViewStateShape } from './diffViewState.svelte';

export type DiffViewState = DiffViewStateShape;

function snapshot(): DiffViewStateShape {
  return { ...diffViewState.state };
}

function createDiffViewStore() {
  const mirror = writable<DiffViewStateShape>(snapshot());
  const refresh = () => mirror.set(snapshot());

  return {
    subscribe: mirror.subscribe,

    open: (projectPath: string) => {
      diffViewState.open(projectPath);
      refresh();
    },

    close: () => {
      diffViewState.close();
      refresh();
    },
  };
}

export const diffViewStore = createDiffViewStore();

export const isDiffViewOpen = derived(diffViewStore, ($diffView) => $diffView.isOpen);
