import { writable } from 'svelte/store';

export type ViewMode = 'terminal' | 'editor';
export type SidebarMode = 'explorer' | 'changes';

export interface AppState {
  sidebarWidth: number;
  showSidebar: boolean;
  currentMode: ViewMode;
  currentFile: string | null;
  sidebarMode: SidebarMode;
}

const initialState: AppState = {
  sidebarWidth: 220,
  showSidebar: true,
  currentMode: 'terminal',
  currentFile: null,
  sidebarMode: 'explorer',
};

function createAppStore() {
  const { subscribe, set, update } = writable<AppState>(initialState);

  return {
    subscribe,
    setSidebarWidth: (width: number) =>
      update((state) => ({
        ...state,
        sidebarWidth: Math.max(160, Math.min(400, width)),
      })),
    toggleSidebar: () =>
      update((state) => ({
        ...state,
        showSidebar: !state.showSidebar,
      })),
    showSidebar: () =>
      update((state) => ({
        ...state,
        showSidebar: true,
      })),
    hideSidebar: () =>
      update((state) => ({
        ...state,
        showSidebar: false,
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
    setSidebarMode: (mode: SidebarMode) =>
      update((state) => ({
        ...state,
        sidebarMode: mode,
      })),
    toggleSidebarMode: () =>
      update((state) => ({
        ...state,
        sidebarMode: state.sidebarMode === 'explorer' ? 'changes' : 'explorer',
      })),
    reset: () => set(initialState),
  };
}

export const appStore = createAppStore();
