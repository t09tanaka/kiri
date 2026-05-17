// Backward-compatible facade over the split settings stores (issue #43).
//
// `settingsStore` historically mixed in-memory UI preferences (fontSize)
// with disk-backed settings (startupCommand). Those responsibilities now
// live in `uiPreferencesStore` and `persistedSettingsStore` respectively.
//
// This file is intentionally kept as a thin facade so existing consumers
// (App.svelte, Terminal.svelte, PeekEditor.svelte, Editor.svelte,
// StartScreen.svelte, plus the test suite) keep working without churn.
// New code should import directly from the underlying stores; once all
// consumers are migrated, this facade can be deleted.
//
// The internal `writable` exists *only* to keep the `subscribe`/`derived`
// surface — it is treated as a fan-out mirror, never as a source of
// truth. Mutations always go through the underlying stores first; the
// mirror is refreshed afterwards so `$settingsStore`, `$fontSize`,
// `$startupCommand`, and `get(settingsStore)` observe the new value.

import { writable, derived } from 'svelte/store';
import { DEFAULT_STARTUP_COMMAND, type StartupCommand } from '@/lib/services/persistenceService';
import { uiPreferencesStore, FONT_SIZE_CONSTRAINTS } from './uiPreferencesStore.svelte';
import { persistedSettingsStore } from './persistedSettingsStore.svelte';

export { FONT_SIZE_CONSTRAINTS };

export interface SettingsState {
  fontSize: number;
  startupCommand: StartupCommand;
}

function snapshot(): SettingsState {
  return {
    fontSize: uiPreferencesStore.fontSize,
    startupCommand: persistedSettingsStore.startupCommand,
  };
}

function createSettingsStore() {
  const mirror = writable<SettingsState>(snapshot());
  const refresh = () => mirror.set(snapshot());

  let unsubscribePersist: (() => void) | null = null;

  /**
   * Once enabled, every state change after the post-restore delay is
   * pushed to `handler` automatically. The 500ms default matches what
   * App.svelte used to do inline: it avoids saving the snapshot we just
   * hydrated from the persisted store.
   */
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
    unsubscribePersist = mirror.subscribe((state) => {
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
    subscribe: mirror.subscribe,

    enableAutoPersist,

    zoomIn: () => {
      uiPreferencesStore.zoomIn();
      refresh();
    },

    zoomOut: () => {
      uiPreferencesStore.zoomOut();
      refresh();
    },

    resetZoom: () => {
      uiPreferencesStore.resetZoom();
      refresh();
    },

    setStartupCommand: (command: StartupCommand) => {
      persistedSettingsStore.setStartupCommand(command);
      refresh();
    },

    setFontSize: (size: number) => {
      uiPreferencesStore.setFontSize(size);
      refresh();
    },

    getFontSize: (): number => uiPreferencesStore.fontSize,

    /**
     * @deprecated Prefer `enableAutoPersist()` over calling this and
     * piping the result through `saveSettings(...)` by hand. Kept for
     * tests and migration callers.
     */
    getStateForPersistence: (): SettingsState => snapshot(),

    restoreState: (state: Partial<SettingsState>) => {
      uiPreferencesStore.setFontSize(state.fontSize ?? FONT_SIZE_CONSTRAINTS.DEFAULT);
      persistedSettingsStore.setStartupCommand(state.startupCommand ?? DEFAULT_STARTUP_COMMAND);
      refresh();
    },

    reset: () => {
      uiPreferencesStore.reset();
      persistedSettingsStore.reset();
      mirror.set(snapshot());
    },
  };
}

export const settingsStore = createSettingsStore();

export const fontSize = derived(settingsStore, ($settings) => $settings.fontSize);
export const startupCommand = derived(settingsStore, ($settings) => $settings.startupCommand);
