// Disk-backed settings (issue #43 — split from settingsStore).
//
// Responsibility: settings the user expects to survive across windows and
// restarts. Today: the per-pane startup command. Future: anything else we
// hydrate from kiri-settings.json at boot.
//
// Persistence is wired by the App via `settingsStore.enableAutoPersist`
// (the facade); this store does not perform IO itself. Keep it free of
// Tauri/IO imports so it can be exercised in unit tests without mocking
// the store plugin.

import { DEFAULT_STARTUP_COMMAND, type StartupCommand } from '@/lib/services/persistenceService';

class PersistedSettingsStore {
  startupCommand = $state<StartupCommand>(DEFAULT_STARTUP_COMMAND);

  setStartupCommand(command: StartupCommand): void {
    this.startupCommand = command;
  }

  reset(): void {
    this.startupCommand = DEFAULT_STARTUP_COMMAND;
  }
}

export const persistedSettingsStore = new PersistedSettingsStore();
