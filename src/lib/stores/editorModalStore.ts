import { writable, derived } from 'svelte/store';

export interface EditorModalState {
  isOpen: boolean;
  filePath: string | null;
  modified: boolean;
}

const initialState: EditorModalState = {
  isOpen: false,
  filePath: null,
  modified: false,
};

function createEditorModalStore() {
  const { subscribe, set, update } = writable<EditorModalState>(initialState);

  return {
    subscribe,

    /**
     * Open the editor modal with the specified file path
     * @param filePath - Path to the file to edit
     */
    open: (filePath: string) => {
      set({
        isOpen: true,
        filePath,
        modified: false,
      });
    },

    /**
     * Close the editor modal
     */
    close: () => set(initialState),

    /**
     * Set the modified state
     * @param modified - Whether the file has unsaved changes
     */
    setModified: (modified: boolean) => {
      update((state) => ({ ...state, modified }));
    },

    /**
     * Get current modified state
     */
    isModified: (): boolean => {
      let modified = false;
      const unsubscribe = subscribe((state) => {
        modified = state.modified;
      });
      unsubscribe();
      return modified;
    },
  };
}

export const editorModalStore = createEditorModalStore();

// Derived store for checking if editor modal is open
export const isEditorModalOpen = derived(editorModalStore, ($state) => $state.isOpen);
