// Backward-compatible facade over `editorModalState` (issue #42 phase 1).
// See the canonical class in `editorModalState.svelte.ts`.

import { writable, derived } from 'svelte/store';
import { editorModalState, type EditorModalStateShape } from './editorModalState.svelte';

export type EditorModalState = EditorModalStateShape;

function snapshot(): EditorModalStateShape {
  return { ...editorModalState.state };
}

function createEditorModalStore() {
  const mirror = writable<EditorModalStateShape>(snapshot());
  const refresh = () => mirror.set(snapshot());

  return {
    subscribe: mirror.subscribe,

    open: (filePath: string) => {
      editorModalState.open(filePath);
      refresh();
    },

    close: () => {
      editorModalState.close();
      refresh();
    },
  };
}

export const editorModalStore = createEditorModalStore();

export const isEditorModalOpen = derived(editorModalStore, ($state) => $state.isOpen);
