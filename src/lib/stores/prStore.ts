import { writable, derived } from 'svelte/store';
import { prService } from '@/lib/services/prService';
import type { PullRequest } from '@/lib/services/prService';

export interface PrState {
  prs: PullRequest[];
  selectedPr: PullRequest | null;
  isLoading: boolean;
  error: string | null;
  ghAvailable: boolean;
}

const initialState: PrState = {
  prs: [],
  selectedPr: null,
  isLoading: false,
  error: null,
  ghAvailable: false,
};

function createPrStore() {
  const { subscribe, set, update } = writable<PrState>(initialState);

  return {
    subscribe,

    refresh: async (repoPath: string) => {
      update((state) => ({ ...state, isLoading: true, error: null }));

      try {
        const status = await prService.checkGhCli();
        if (!status.installed || !status.authenticated) {
          update((state) => ({
            ...state,
            ghAvailable: false,
            prs: [],
            isLoading: false,
          }));
          return;
        }

        const prs = await prService.listPrs(repoPath);
        update((state) => ({
          ...state,
          prs,
          ghAvailable: true,
          isLoading: false,
        }));
      } catch (e) {
        update((state) => ({
          ...state,
          error: e instanceof Error ? e.message : String(e),
          isLoading: false,
        }));
      }
    },

    selectPr: async (repoPath: string, number: number) => {
      update((state) => ({ ...state, error: null }));
      try {
        const pr = await prService.getPrDetail(repoPath, number);
        update((state) => ({ ...state, selectedPr: pr }));
      } catch (e) {
        update((state) => ({
          ...state,
          selectedPr: null,
          error: e instanceof Error ? e.message : String(e),
        }));
      }
    },

    clearSelection: () => {
      update((state) => ({ ...state, selectedPr: null }));
    },

    clear: () => set(initialState),
  };
}

export const prStore = createPrStore();

export const prCount = derived(prStore, ($prStore) => $prStore.prs.length);
export const hasPrs = derived(prStore, ($prStore) => $prStore.prs.length > 0);
