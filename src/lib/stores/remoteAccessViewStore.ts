import { writable } from 'svelte/store';

interface RemoteAccessViewState {
  isSettingsOpen: boolean;
  isQrModalOpen: boolean;
}

function createRemoteAccessViewStore() {
  const { subscribe, update } = writable<RemoteAccessViewState>({
    isSettingsOpen: false,
    isQrModalOpen: false,
  });

  return {
    subscribe,
    openSettings: () => update((s) => ({ ...s, isSettingsOpen: true })),
    closeSettings: () => update((s) => ({ ...s, isSettingsOpen: false })),
    toggleSettings: () => update((s) => ({ ...s, isSettingsOpen: !s.isSettingsOpen })),
    openQrModal: () => update((s) => ({ ...s, isQrModalOpen: true })),
    closeQrModal: () => update((s) => ({ ...s, isQrModalOpen: false })),
  };
}

export const remoteAccessViewStore = createRemoteAccessViewStore();
