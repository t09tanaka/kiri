import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export interface FileSearchResult {
  path: string;
  name: string;
  is_dir: boolean;
  score: number;
}

export interface ContentMatch {
  line: number;
  content: string;
  start: number;
  end: number;
}

export interface ContentSearchResult {
  path: string;
  name: string;
  matches: ContentMatch[];
}

interface SearchState {
  isQuickOpenVisible: boolean;
  isSearchPanelVisible: boolean;
  fileResults: FileSearchResult[];
  contentResults: ContentSearchResult[];
  isSearching: boolean;
  selectedIndex: number;
  rootPath: string;
}

function createSearchStore() {
  const { subscribe, update } = writable<SearchState>({
    isQuickOpenVisible: false,
    isSearchPanelVisible: false,
    fileResults: [],
    contentResults: [],
    isSearching: false,
    selectedIndex: 0,
    rootPath: '',
  });

  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  return {
    subscribe,

    setRootPath(path: string) {
      update((state) => ({ ...state, rootPath: path }));
    },

    openQuickOpen() {
      update((state) => ({
        ...state,
        isQuickOpenVisible: true,
        fileResults: [],
        selectedIndex: 0,
      }));
    },

    closeQuickOpen() {
      update((state) => ({
        ...state,
        isQuickOpenVisible: false,
        fileResults: [],
        selectedIndex: 0,
      }));
    },

    toggleSearchPanel() {
      update((state) => ({
        ...state,
        isSearchPanelVisible: !state.isSearchPanelVisible,
      }));
    },

    closeSearchPanel() {
      update((state) => ({
        ...state,
        isSearchPanelVisible: false,
        contentResults: [],
      }));
    },

    async searchFiles(query: string) {
      if (searchTimeout) {
        clearTimeout(searchTimeout);
      }

      if (query.length === 0) {
        update((state) => ({
          ...state,
          fileResults: [],
          selectedIndex: 0,
        }));
        return;
      }

      update((state) => ({ ...state, isSearching: true }));

      searchTimeout = setTimeout(async () => {
        try {
          let rootPath = '';
          update((s) => {
            rootPath = s.rootPath;
            return s;
          });

          if (!rootPath) {
            rootPath = await invoke<string>('get_home_directory');
          }

          const results = await invoke<FileSearchResult[]>('search_files', {
            rootPath,
            query,
            maxResults: 50,
          });

          update((state) => ({
            ...state,
            fileResults: results,
            isSearching: false,
            selectedIndex: 0,
          }));
        } catch (error) {
          console.error('File search failed:', error);
          update((state) => ({ ...state, isSearching: false }));
        }
      }, 100);
    },

    async searchContent(query: string) {
      if (query.length < 2) {
        update((state) => ({ ...state, contentResults: [] }));
        return;
      }

      update((state) => ({ ...state, isSearching: true }));

      try {
        let rootPath = '';
        update((s) => {
          rootPath = s.rootPath;
          return s;
        });

        if (!rootPath) {
          rootPath = await invoke<string>('get_home_directory');
        }

        const results = await invoke<ContentSearchResult[]>('search_content', {
          rootPath,
          query,
          maxResults: 50,
          excludePatterns: [],
        });

        update((state) => ({
          ...state,
          contentResults: results,
          isSearching: false,
        }));
      } catch (error) {
        console.error('Content search failed:', error);
        update((state) => ({ ...state, isSearching: false }));
      }
    },

    selectPrevious() {
      update((state) => ({
        ...state,
        selectedIndex: Math.max(0, state.selectedIndex - 1),
      }));
    },

    selectNext() {
      update((state) => ({
        ...state,
        selectedIndex: Math.min(state.fileResults.length - 1, state.selectedIndex + 1),
      }));
    },

    getSelectedFile(): FileSearchResult | null {
      let result: FileSearchResult | null = null;
      update((state) => {
        if (state.fileResults.length > 0 && state.selectedIndex < state.fileResults.length) {
          result = state.fileResults[state.selectedIndex];
        }
        return state;
      });
      return result;
    },
  };
}

export const searchStore = createSearchStore();

export const isQuickOpenVisible = derived(searchStore, ($store) => $store.isQuickOpenVisible);

export const isSearchPanelVisible = derived(searchStore, ($store) => $store.isSearchPanelVisible);
