import { writable, derived, get } from 'svelte/store';
import { DEFAULT_STARTUP_COMMAND, type StartupCommand } from '@/lib/services/persistenceService';

// Font size constraints (VSCode-like)
const MIN_FONT_SIZE = 8;
const MAX_FONT_SIZE = 32;
const DEFAULT_FONT_SIZE = 13;
const ZOOM_STEP = 1;

export interface SettingsState {
  fontSize: number;
  startupCommand: StartupCommand;
}

const initialState: SettingsState = {
  fontSize: DEFAULT_FONT_SIZE,
  startupCommand: DEFAULT_STARTUP_COMMAND,
};

function createSettingsStore() {
  const { subscribe, set, update } = writable<SettingsState>(initialState);

  /**
   * Once enabled, every state change after `restoreState` finishes is
   * pushed to `handler` automatically, so callers never have to
   * remember to invoke `saveSettings(getStateForPersistence())` after
   * a mutation. Returns the unsubscribe function so the App can tear
   * the auto-save down on window close.
   *
   * The 500ms delay before arming `ready` matches what App.svelte
   * used to do inline: it avoids saving the snapshot we just hydrated
   * from the persisted store.
   */
  let unsubscribePersist: (() => void) | null = null;
  function enableAutoPersist(
    handler: (state: SettingsState) => unknown,
    options: { delayMs?: number } = {}
  ): () => void {
    if (unsubscribePersist) unsubscribePersist();
    let ready = false;
    const delayMs = options.delayMs ?? 500;
    const timer = setTimeout(() => {
      ready = true;
    }, delayMs);
    unsubscribePersist = subscribe((state) => {
      if (!ready) return;
      void handler(state);
    });
    return () => {
      clearTimeout(timer);
      if (unsubscribePersist) {
        unsubscribePersist();
        unsubscribePersist = null;
      }
    };
  }

  return {
    subscribe,

    /**
     * Wire up auto-persistence. Pass the same `saveSettings(state)`
     * that callers used to invoke manually; from then on every store
     * mutation - existing or future field - is pushed to the handler
     * without the caller having to remember.
     */
    enableAutoPersist,

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
     * Set startup command
     */
    setStartupCommand: (command: StartupCommand) => {
      update((state) => ({
        ...state,
        startupCommand: command,
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
     * @deprecated Prefer `enableAutoPersist()` over calling this and
     * piping the result through `saveSettings(...)` by hand. Kept for
     * tests and migration callers; the App wires `enableAutoPersist`
     * during boot so mutations persist automatically.
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
        startupCommand: state.startupCommand ?? DEFAULT_STARTUP_COMMAND,
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
export const startupCommand = derived(settingsStore, ($settings) => $settings.startupCommand);

// Export constants for testing
export const FONT_SIZE_CONSTRAINTS = {
  MIN: MIN_FONT_SIZE,
  MAX: MAX_FONT_SIZE,
  DEFAULT: DEFAULT_FONT_SIZE,
  STEP: ZOOM_STEP,
};
