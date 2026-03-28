import { writable, derived } from 'svelte/store';

export interface PrViewState {
  isOpen: boolean;
  projectPath: string | null;
}

const initialState: PrViewState = {
  isOpen: false,
  projectPath: null,
};

function createPrViewStore() {
  const { subscribe, set } = writable<PrViewState>(initialState);

  return {
    subscribe,
    open: (projectPath: string) => set({ isOpen: true, projectPath }),
    close: () => set(initialState),
  };
}

export const prViewStore = createPrViewStore();

export const isPrViewOpen = derived(prViewStore, ($prView) => $prView.isOpen);
