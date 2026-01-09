import { writable } from 'svelte/store';

export type ViewMode = 'terminal' | 'editor';

export interface AppState {
  sidebarWidth: number;
  currentMode: ViewMode;
  currentFile: string | null;
}

const initialState: AppState = {
  sidebarWidth: 200,
  currentMode: 'terminal',
  currentFile: null,
};

function createAppStore() {
  const { subscribe, set, update } = writable<AppState>(initialState);

  return {
    subscribe,
    setSidebarWidth: (width: number) =>
      update((state) => ({
        ...state,
        sidebarWidth: Math.max(150, Math.min(400, width)),
      })),
    setMode: (mode: ViewMode) =>
      update((state) => ({
        ...state,
        currentMode: mode,
      })),
    setCurrentFile: (file: string | null) =>
      update((state) => ({
        ...state,
        currentFile: file,
      })),
    reset: () => set(initialState),
  };
}

export const appStore = createAppStore();
