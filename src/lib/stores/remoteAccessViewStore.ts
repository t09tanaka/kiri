import { writable } from 'svelte/store';

interface RemoteAccessViewState {
  isOpen: boolean;
}

function createRemoteAccessViewStore() {
  const { subscribe, set, update } = writable<RemoteAccessViewState>({
    isOpen: false,
  });

  return {
    subscribe,
    open: () => update((s) => ({ ...s, isOpen: true })),
    close: () => set({ isOpen: false }),
    toggle: () => update((s) => ({ ...s, isOpen: !s.isOpen })),
  };
}

export const remoteAccessViewStore = createRemoteAccessViewStore();
