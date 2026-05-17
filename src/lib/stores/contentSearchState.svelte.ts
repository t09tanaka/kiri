// Canonical $state class for the content search modal (issue #42 phase 1).
//
// See `contentSearchStore.ts` (the facade) for the legacy export. This
// class owns the in-memory state; the facade handles fan-out to the
// existing `$store` / `derived(...)` consumers.
//
// Async behaviors (debounced search, project-settings persistence) stay
// in the facade because they need to drive the writable mirror and call
// into services. Once consumers migrate to read `contentSearchState`
// directly, those behaviors can move down here.

import type { ContentSearchResult, ContentMatch } from '@/lib/services/searchService';
import { DEFAULT_EXCLUDE_PATTERNS } from '@/lib/services/persistenceService';

export type { ContentSearchResult, ContentMatch };

export interface ContentSearchStateShape {
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

export function initialContentSearchState(): ContentSearchStateShape {
  return {
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
}

class ContentSearchState {
  state = $state<ContentSearchStateShape>(initialContentSearchState());

  patch(partial: Partial<ContentSearchStateShape>): void {
    this.state = { ...this.state, ...partial };
  }

  reset(): void {
    this.state = initialContentSearchState();
  }
}

export const contentSearchState = new ContentSearchState();
