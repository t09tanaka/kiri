import { writable, derived } from 'svelte/store';

export interface EditorModalState {
  isOpen: boolean;
  filePath: string | null;
}

const initialState: EditorModalState = {
  isOpen: false,
  filePath: null,
};

function createEditorModalStore() {
  const { subscribe, set } = writable<EditorModalState>(initialState);

  return {
    subscribe,

    /**
     * Open the file viewer modal with the specified file path
     * @param filePath - Path to the file to view
     */
    open: (filePath: string) => {
      set({
        isOpen: true,
        filePath,
      });
    },

    /**
     * Close the file viewer modal
     */
    close: () => set(initialState),
  };
}

export const editorModalStore = createEditorModalStore();

// Derived store for checking if editor modal is open
export const isEditorModalOpen = derived(editorModalStore, ($state) => $state.isOpen);
