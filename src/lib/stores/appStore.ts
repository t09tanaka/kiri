import { writable, get } from 'svelte/store';
import type { PersistedUI } from '@/lib/services/persistenceService';

export type ViewMode = 'terminal' | 'editor';
// Keep 'changes' in type for backwards compatibility with persisted data
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

    /**
     * Get UI state for persistence
     */
    getUIForPersistence: (): PersistedUI => {
      const state = get({ subscribe });
      return {
        sidebarWidth: state.sidebarWidth,
        showSidebar: state.showSidebar,
        sidebarMode: state.sidebarMode,
      };
    },

    /**
     * Restore UI state from persistence
     * Note: sidebarMode is always set to 'explorer' as DiffView now opens in a separate window
     */
    restoreUI: (ui: PersistedUI) => {
      update((state) => ({
        ...state,
        sidebarWidth: Math.max(160, Math.min(400, ui.sidebarWidth)),
        showSidebar: ui.showSidebar,
        sidebarMode: 'explorer',
      }));
    },

    reset: () => set(initialState),
  };
}

export const appStore = createAppStore();
