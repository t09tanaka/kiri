// Backward-compatible facade over `contentSearchState` (issue #42 phase 1).
//
// The canonical state lives in `contentSearchState.svelte.ts`. This file
// keeps the public surface intact: `subscribe` (writable contract), the
// `derived(...)` exports below, and the async methods (`search`,
// `addExcludePattern`, etc.) that drive both the canonical state and
// the legacy writable mirror.

import { writable, derived } from 'svelte/store';
import { searchService } from '@/lib/services/searchService';
import {
  loadProjectSettings,
  saveProjectSettings,
  DEFAULT_EXCLUDE_PATTERNS,
  type ProjectSettings,
} from '@/lib/services/persistenceService';
import {
  contentSearchState,
  initialContentSearchState,
  type ContentSearchStateShape,
  type ContentSearchResult,
  type ContentMatch,
} from './contentSearchState.svelte';

export type { ContentSearchResult, ContentMatch };

function snapshot(): ContentSearchStateShape {
  return { ...contentSearchState.state };
}

function createContentSearchStore() {
  const mirror = writable<ContentSearchStateShape>(snapshot());
  const refresh = () => mirror.set(snapshot());

  const patch = (partial: Partial<ContentSearchStateShape>) => {
    contentSearchState.patch(partial);
    refresh();
  };

  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  let lastProjectPath: string | null = null;

  const store = {
    subscribe: mirror.subscribe,

    async open(projectPath: string) {
      if (lastProjectPath !== projectPath) {
        lastProjectPath = projectPath;
        const settings = await loadProjectSettings(projectPath);
        patch({
          isOpen: true,
          projectPath,
          query: '',
          results: [],
          selectedFileIndex: 0,
          selectedMatchIndex: 0,
          excludePatterns: settings.searchExcludePatterns,
          isSettingsOpen: false,
          error: null,
        });
      } else {
        patch({
          isOpen: true,
          projectPath,
          query: '',
          results: [],
          selectedFileIndex: 0,
          selectedMatchIndex: 0,
          isSettingsOpen: false,
          error: null,
        });
      }
    },

    close() {
      if (searchTimeout) {
        clearTimeout(searchTimeout);
        searchTimeout = null;
      }
      patch({ isOpen: false, isSearching: false, isSettingsOpen: false });
    },

    async toggle(projectPath: string) {
      if (contentSearchState.state.isOpen) {
        this.close();
      } else {
        await this.open(projectPath);
      }
    },

    search(query: string) {
      if (searchTimeout) {
        clearTimeout(searchTimeout);
      }

      patch({ query });

      if (query.length < 2) {
        patch({ results: [], selectedFileIndex: 0, selectedMatchIndex: 0, isSearching: false });
        return;
      }

      patch({ isSearching: true, error: null });

      searchTimeout = setTimeout(async () => {
        const current = contentSearchState.state;
        if (!current.projectPath) {
          patch({ isSearching: false, error: 'No project path' });
          return;
        }

        try {
          const results = await searchService.searchContent(
            current.projectPath,
            query,
            100,
            current.excludePatterns
          );
          patch({
            results,
            isSearching: false,
            selectedFileIndex: 0,
            selectedMatchIndex: 0,
            error: null,
          });
        } catch (error) {
          console.error('Content search failed:', error);
          patch({
            isSearching: false,
            error: error instanceof Error ? error.message : 'Search failed',
          });
        }
      }, 200);
    },

    selectPreviousFile() {
      const s = contentSearchState.state;
      patch({
        selectedFileIndex: Math.max(0, s.selectedFileIndex - 1),
        selectedMatchIndex: 0,
      });
    },

    selectNextFile() {
      const s = contentSearchState.state;
      patch({
        selectedFileIndex: Math.min(s.results.length - 1, s.selectedFileIndex + 1),
        selectedMatchIndex: 0,
      });
    },

    selectPreviousMatch() {
      const s = contentSearchState.state;
      const currentFile = s.results[s.selectedFileIndex];
      if (!currentFile) return;
      patch({ selectedMatchIndex: Math.max(0, s.selectedMatchIndex - 1) });
    },

    selectNextMatch() {
      const s = contentSearchState.state;
      const currentFile = s.results[s.selectedFileIndex];
      if (!currentFile) return;
      patch({
        selectedMatchIndex: Math.min(currentFile.matches.length - 1, s.selectedMatchIndex + 1),
      });
    },

    selectFile(index: number) {
      const s = contentSearchState.state;
      patch({
        selectedFileIndex: Math.max(0, Math.min(s.results.length - 1, index)),
        selectedMatchIndex: 0,
      });
    },

    getSelectedFile(): ContentSearchResult | null {
      const s = contentSearchState.state;
      return s.results[s.selectedFileIndex] ?? null;
    },

    getSelectedMatch(): ContentMatch | null {
      const s = contentSearchState.state;
      const file = s.results[s.selectedFileIndex];
      if (!file) return null;
      return file.matches[s.selectedMatchIndex] ?? null;
    },

    toggleSettings() {
      patch({ isSettingsOpen: !contentSearchState.state.isSettingsOpen });
    },

    closeSettings() {
      patch({ isSettingsOpen: false });
    },

    async addExcludePattern(pattern: string) {
      const trimmed = pattern.trim();
      const current = contentSearchState.state;
      if (!trimmed || current.excludePatterns.includes(trimmed)) return;

      const newPatterns = [...current.excludePatterns, trimmed];
      patch({ excludePatterns: newPatterns });

      if (current.projectPath) {
        await saveProjectSettings(current.projectPath, {
          searchExcludePatterns: newPatterns,
        });
      }

      if (current.query.length >= 2) {
        this.search(current.query);
      }
    },

    async removeExcludePattern(pattern: string) {
      const current = contentSearchState.state;
      const newPatterns = current.excludePatterns.filter((p) => p !== pattern);
      patch({ excludePatterns: newPatterns });

      if (current.projectPath) {
        await saveProjectSettings(current.projectPath, {
          searchExcludePatterns: newPatterns,
        });
      }

      if (current.query.length >= 2) {
        this.search(current.query);
      }
    },

    async resetExcludePatterns() {
      const current = contentSearchState.state;
      const newPatterns = [...DEFAULT_EXCLUDE_PATTERNS];
      patch({ excludePatterns: newPatterns });

      if (current.projectPath) {
        await saveProjectSettings(current.projectPath, {
          searchExcludePatterns: newPatterns,
        } as ProjectSettings);
      }

      if (current.query.length >= 2) {
        this.search(current.query);
      }
    },

    reset() {
      if (searchTimeout) {
        clearTimeout(searchTimeout);
        searchTimeout = null;
      }
      lastProjectPath = null;
      contentSearchState.reset();
      mirror.set(initialContentSearchState());
    },
  };

  return store;
}

export const contentSearchStore = createContentSearchStore();

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
