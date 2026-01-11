import { writable, derived } from 'svelte/store';

export interface PeekState {
  isOpen: boolean;
  filePath: string | null;
  lineNumber?: number;
  columnNumber?: number;
}

const initialState: PeekState = {
  isOpen: false,
  filePath: null,
  lineNumber: undefined,
  columnNumber: undefined,
};

function createPeekStore() {
  const { subscribe, set } = writable<PeekState>(initialState);

  return {
    subscribe,

    /**
     * Open the peek editor with the specified file
     * @param filePath - Path to the file to display
     * @param lineNumber - Optional line number to scroll to (1-indexed)
     * @param columnNumber - Optional column number
     */
    open: (filePath: string, lineNumber?: number, columnNumber?: number) => {
      set({
        isOpen: true,
        filePath,
        lineNumber,
        columnNumber,
      });
    },

    /**
     * Close the peek editor
     */
    close: () => set(initialState),
  };
}

export const peekStore = createPeekStore();

// Derived store for checking if peek is open
export const isPeekOpen = derived(peekStore, ($peek) => $peek.isOpen);
