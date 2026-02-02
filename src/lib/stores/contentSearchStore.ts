import { writable, derived, get } from 'svelte/store';
import {
  searchService,
  type ContentSearchResult,
  type ContentMatch,
} from '@/lib/services/searchService';
import {
  loadProjectSettings,
  saveProjectSettings,
  DEFAULT_EXCLUDE_PATTERNS,
  type ProjectSettings,
} from '@/lib/services/persistenceService';

/**
 * Re-export types for convenience
 */
export type { ContentSearchResult, ContentMatch };

interface ContentSearchState {
  isOpen: boolean;
  projectPath: string | null;
  query: string;
  results: ContentSearchResult[];
  isSearching: boolean;
  selectedFileIndex: number;
  selectedMatchIndex: number;
  excludePatterns: string[];
  isSettingsOpen: boolean;
  error: string | null;
}

const initialState: ContentSearchState = {
  isOpen: false,
  projectPath: null,
  query: '',
  results: [],
  isSearching: false,
  selectedFileIndex: 0,
  selectedMatchIndex: 0,
  excludePatterns: [...DEFAULT_EXCLUDE_PATTERNS],
  isSettingsOpen: false,
  error: null,
};

function createContentSearchStore() {
  const { subscribe, update, set } = writable<ContentSearchState>(initialState);

  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  let lastProjectPath: string | null = null;

  return {
    subscribe,

    /**
     * Open the content search modal for a project
     */
    async open(projectPath: string) {
      // Load project settings if different project
      if (lastProjectPath !== projectPath) {
        lastProjectPath = projectPath;
        const settings = await loadProjectSettings(projectPath);
        update((state) => ({
          ...state,
          isOpen: true,
          projectPath,
          query: '',
          results: [],
          selectedFileIndex: 0,
          selectedMatchIndex: 0,
          excludePatterns: settings.searchExcludePatterns,
          isSettingsOpen: false,
          error: null,
        }));
      } else {
        update((state) => ({
          ...state,
          isOpen: true,
          projectPath,
          query: '',
          results: [],
          selectedFileIndex: 0,
          selectedMatchIndex: 0,
          isSettingsOpen: false,
          error: null,
        }));
      }
    },

    /**
     * Close the content search modal
     */
    close() {
      if (searchTimeout) {
        clearTimeout(searchTimeout);
        searchTimeout = null;
      }
      update((state) => ({
        ...state,
        isOpen: false,
        isSearching: false,
        isSettingsOpen: false,
      }));
    },

    /**
     * Toggle the content search modal
     */
    async toggle(projectPath: string) {
      const state = get({ subscribe });
      if (state.isOpen) {
        this.close();
      } else {
        await this.open(projectPath);
      }
    },

    /**
     * Perform content search with debouncing
     */
    search(query: string) {
      if (searchTimeout) {
        clearTimeout(searchTimeout);
      }

      update((state) => ({ ...state, query }));

      if (query.length < 2) {
        update((state) => ({
          ...state,
          results: [],
          selectedFileIndex: 0,
          selectedMatchIndex: 0,
          isSearching: false,
        }));
        return;
      }

      update((state) => ({ ...state, isSearching: true, error: null }));

      searchTimeout = setTimeout(async () => {
        const state = get({ subscribe });
        if (!state.projectPath) {
          update((s) => ({ ...s, isSearching: false, error: 'No project path' }));
          return;
        }

        try {
          const results = await searchService.searchContent(
            state.projectPath,
            query,
            100,
            state.excludePatterns
          );

          update((s) => ({
            ...s,
            results,
            isSearching: false,
            selectedFileIndex: 0,
            selectedMatchIndex: 0,
            error: null,
          }));
        } catch (error) {
          console.error('Content search failed:', error);
          update((s) => ({
            ...s,
            isSearching: false,
            error: error instanceof Error ? error.message : 'Search failed',
          }));
        }
      }, 200);
    },

    /**
     * Select previous file in results
     */
    selectPreviousFile() {
      update((state) => ({
        ...state,
        selectedFileIndex: Math.max(0, state.selectedFileIndex - 1),
        selectedMatchIndex: 0,
      }));
    },

    /**
     * Select next file in results
     */
    selectNextFile() {
      update((state) => ({
        ...state,
        selectedFileIndex: Math.min(state.results.length - 1, state.selectedFileIndex + 1),
        selectedMatchIndex: 0,
      }));
    },

    /**
     * Select previous match in current file
     */
    selectPreviousMatch() {
      update((state) => {
        const currentFile = state.results[state.selectedFileIndex];
        if (!currentFile) return state;
        return {
          ...state,
          selectedMatchIndex: Math.max(0, state.selectedMatchIndex - 1),
        };
      });
    },

    /**
     * Select next match in current file
     */
    selectNextMatch() {
      update((state) => {
        const currentFile = state.results[state.selectedFileIndex];
        if (!currentFile) return state;
        return {
          ...state,
          selectedMatchIndex: Math.min(
            currentFile.matches.length - 1,
            state.selectedMatchIndex + 1
          ),
        };
      });
    },

    /**
     * Select a specific file by index
     */
    selectFile(index: number) {
      update((state) => ({
        ...state,
        selectedFileIndex: Math.max(0, Math.min(state.results.length - 1, index)),
        selectedMatchIndex: 0,
      }));
    },

    /**
     * Get the currently selected file
     */
    getSelectedFile(): ContentSearchResult | null {
      const state = get({ subscribe });
      return state.results[state.selectedFileIndex] ?? null;
    },

    /**
     * Get the currently selected match
     */
    getSelectedMatch(): ContentMatch | null {
      const state = get({ subscribe });
      const file = state.results[state.selectedFileIndex];
      if (!file) return null;
      return file.matches[state.selectedMatchIndex] ?? null;
    },

    /**
     * Toggle settings panel
     */
    toggleSettings() {
      update((state) => ({ ...state, isSettingsOpen: !state.isSettingsOpen }));
    },

    /**
     * Close settings panel
     */
    closeSettings() {
      update((state) => ({ ...state, isSettingsOpen: false }));
    },

    /**
     * Add an exclude pattern
     */
    async addExcludePattern(pattern: string) {
      const state = get({ subscribe });
      if (!pattern.trim() || state.excludePatterns.includes(pattern.trim())) {
        return;
      }

      const newPatterns = [...state.excludePatterns, pattern.trim()];
      update((s) => ({ ...s, excludePatterns: newPatterns }));

      // Save to project settings
      if (state.projectPath) {
        await saveProjectSettings(state.projectPath, {
          searchExcludePatterns: newPatterns,
        });
      }

      // Re-run search with new patterns
      if (state.query.length >= 2) {
        this.search(state.query);
      }
    },

    /**
     * Remove an exclude pattern
     */
    async removeExcludePattern(pattern: string) {
      const state = get({ subscribe });
      const newPatterns = state.excludePatterns.filter((p) => p !== pattern);
      update((s) => ({ ...s, excludePatterns: newPatterns }));

      // Save to project settings
      if (state.projectPath) {
        await saveProjectSettings(state.projectPath, {
          searchExcludePatterns: newPatterns,
        });
      }

      // Re-run search with new patterns
      if (state.query.length >= 2) {
        this.search(state.query);
      }
    },

    /**
     * Reset exclude patterns to defaults
     */
    async resetExcludePatterns() {
      const state = get({ subscribe });
      const newPatterns = [...DEFAULT_EXCLUDE_PATTERNS];
      update((s) => ({ ...s, excludePatterns: newPatterns }));

      // Save to project settings
      if (state.projectPath) {
        await saveProjectSettings(state.projectPath, {
          searchExcludePatterns: newPatterns,
        } as ProjectSettings);
      }

      // Re-run search with new patterns
      if (state.query.length >= 2) {
        this.search(state.query);
      }
    },

    /**
     * Reset the store to initial state
     */
    reset() {
      if (searchTimeout) {
        clearTimeout(searchTimeout);
        searchTimeout = null;
      }
      lastProjectPath = null;
      set(initialState);
    },
  };
}

export const contentSearchStore = createContentSearchStore();

// Derived stores for easy access
export const isContentSearchOpen = derived(contentSearchStore, ($store) => $store.isOpen);

export const contentSearchResults = derived(contentSearchStore, ($store) => $store.results);

export const isContentSearching = derived(contentSearchStore, ($store) => $store.isSearching);

export const contentSearchQuery = derived(contentSearchStore, ($store) => $store.query);

export const selectedContentFile = derived(
  contentSearchStore,
  ($store) => $store.results[$store.selectedFileIndex] ?? null
);

export const isContentSearchSettingsOpen = derived(
  contentSearchStore,
  ($store) => $store.isSettingsOpen
);
