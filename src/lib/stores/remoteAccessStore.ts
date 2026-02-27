import { writable, derived } from 'svelte/store';

export interface RemoteAccessState {
  serverRunning: boolean;
  tunnelRunning: boolean;
  tunnelUrl: string | null;
  port: number;
  hasToken: boolean;
}

const DEFAULT_PORT = 9876;

const initialState: RemoteAccessState = {
  serverRunning: false,
  tunnelRunning: false,
  tunnelUrl: null,
  port: DEFAULT_PORT,
  hasToken: false,
};

function createRemoteAccessStore() {
  const { subscribe, set, update } = writable<RemoteAccessState>(initialState);

  return {
    subscribe,

    /**
     * Set whether the remote access server is running
     */
    setServerRunning: (running: boolean) => update((s) => ({ ...s, serverRunning: running })),

    /**
     * Set whether the Cloudflare tunnel is running, with optional URL
     */
    setTunnelRunning: (running: boolean, url?: string) =>
      update((s) => ({ ...s, tunnelRunning: running, tunnelUrl: url ?? null })),

    /**
     * Set the server port
     */
    setPort: (port: number) => update((s) => ({ ...s, port })),

    /**
     * Set whether an authentication token exists
     */
    setHasToken: (has: boolean) => update((s) => ({ ...s, hasToken: has })),

    /**
     * Reset to initial state
     */
    reset: () => set(initialState),
  };
}

export const remoteAccessStore = createRemoteAccessStore();

// Derived stores for convenience
export const isRemoteActive = derived(remoteAccessStore, ($s) => $s.serverRunning);
