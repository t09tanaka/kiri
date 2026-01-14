import { writable, derived, get } from 'svelte/store';

// Font size constraints (VSCode-like)
const MIN_FONT_SIZE = 8;
const MAX_FONT_SIZE = 32;
const DEFAULT_FONT_SIZE = 13;
const ZOOM_STEP = 1;

export interface SettingsState {
  fontSize: number;
}

const initialState: SettingsState = {
  fontSize: DEFAULT_FONT_SIZE,
};

function createSettingsStore() {
  const { subscribe, set, update } = writable<SettingsState>(initialState);

  return {
    subscribe,

    /**
     * Zoom in (increase font size)
     */
    zoomIn: () => {
      update((state) => ({
        ...state,
        fontSize: Math.min(state.fontSize + ZOOM_STEP, MAX_FONT_SIZE),
      }));
    },

    /**
     * Zoom out (decrease font size)
     */
    zoomOut: () => {
      update((state) => ({
        ...state,
        fontSize: Math.max(state.fontSize - ZOOM_STEP, MIN_FONT_SIZE),
      }));
    },

    /**
     * Reset zoom to default
     */
    resetZoom: () => {
      update((state) => ({
        ...state,
        fontSize: DEFAULT_FONT_SIZE,
      }));
    },

    /**
     * Set font size directly
     */
    setFontSize: (size: number) => {
      const clampedSize = Math.max(MIN_FONT_SIZE, Math.min(size, MAX_FONT_SIZE));
      update((state) => ({
        ...state,
        fontSize: clampedSize,
      }));
    },

    /**
     * Get current font size
     */
    getFontSize: (): number => {
      return get({ subscribe }).fontSize;
    },

    /**
     * Get state for persistence
     */
    getStateForPersistence: (): SettingsState => {
      return get({ subscribe });
    },

    /**
     * Restore state from persistence
     */
    restoreState: (state: Partial<SettingsState>) => {
      update((current) => ({
        ...current,
        fontSize: state.fontSize ?? DEFAULT_FONT_SIZE,
      }));
    },

    /**
     * Reset to initial state
     */
    reset: () => set(initialState),
  };
}

export const settingsStore = createSettingsStore();

// Derived stores for convenience
export const fontSize = derived(settingsStore, ($settings) => $settings.fontSize);

// Export constants for testing
export const FONT_SIZE_CONSTRAINTS = {
  MIN: MIN_FONT_SIZE,
  MAX: MAX_FONT_SIZE,
  DEFAULT: DEFAULT_FONT_SIZE,
  STEP: ZOOM_STEP,
};
