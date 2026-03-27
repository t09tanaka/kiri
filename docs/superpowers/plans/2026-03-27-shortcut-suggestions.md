# Shortcut Suggestions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Auto-detect repeated Claude CLI inputs and suggest adding them as terminal shortcuts.

**Architecture:** Frontend-only approach. A new `inputStatsService` captures Enter-delimited text from the terminal `onData` handler (only when AI is running), aggregates usage stats, and surfaces suggestions via a badge + popover on the existing shortcut bar. Stats persist in `kiri-settings.json` via the existing Tauri Store API.

**Tech Stack:** Svelte 5, TypeScript, Vitest, @testing-library/svelte

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `src/lib/services/inputStatsService.ts` | Record inputs, aggregate stats, evaluate suggestion conditions, manage suggestion queue |
| `src/lib/services/inputStatsService.test.ts` | Unit tests for all service logic |
| `src/lib/components/terminal/ShortcutSuggestions.svelte` | Badge + popover UI for suggestions |
| `src/lib/components/terminal/ShortcutSuggestions.browser.test.ts` | Browser tests for UI component |

### Modified Files

| File | Change |
|------|--------|
| `src/lib/services/persistenceService.ts` | Add `loadInputStats()` / `saveInputStats()` |
| `src/lib/services/persistenceService.shortcuts.test.ts` | Add tests for new persistence functions (or new test file) |
| `src/lib/components/terminal/Terminal.svelte` | Add input buffer in `onData`, mount `ShortcutSuggestions`, wire up handlers |
| `src/lib/components/terminal/TerminalShortcutBar.svelte` | Add suggestion badge slot next to settings button |

---

### Task 1: Add `InputRecord` type and persistence functions

**Files:**
- Modify: `src/lib/services/persistenceService.ts`
- Create: `src/lib/services/persistenceService.inputStats.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/lib/services/persistenceService.inputStats.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Store } from '@tauri-apps/plugin-store';

beforeEach(() => {
  vi.resetModules();
});

async function importModule() {
  return await import('./persistenceService');
}

describe('Input Stats Persistence', () => {
  let mockStore: {
    get: ReturnType<typeof vi.fn>;
    set: ReturnType<typeof vi.fn>;
    save: ReturnType<typeof vi.fn>;
    reload: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    mockStore = {
      get: vi.fn().mockResolvedValue(null),
      set: vi.fn().mockResolvedValue(undefined),
      save: vi.fn().mockResolvedValue(undefined),
      reload: vi.fn().mockResolvedValue(undefined),
    };
    vi.mocked(Store.load).mockResolvedValue(mockStore as unknown as Store);
  });

  describe('loadInputStats', () => {
    it('should return empty array when no stats are stored', async () => {
      const { loadInputStats } = await importModule();
      const result = await loadInputStats();
      expect(result).toEqual([]);
      expect(mockStore.reload).toHaveBeenCalled();
      expect(mockStore.get).toHaveBeenCalledWith('inputStats');
    });

    it('should return stored stats', async () => {
      const stats = [
        { text: 'hello', rawText: 'Hello', count: 3, lastUsed: 1000, firstSeen: 500, dismissedAt: null },
      ];
      mockStore.get.mockResolvedValue(stats);
      const { loadInputStats } = await importModule();
      const result = await loadInputStats();
      expect(result).toEqual(stats);
    });

    it('should return empty array on error', async () => {
      mockStore.get.mockRejectedValue(new Error('fail'));
      const { loadInputStats } = await importModule();
      const result = await loadInputStats();
      expect(result).toEqual([]);
    });
  });

  describe('saveInputStats', () => {
    it('should save stats to store', async () => {
      const stats = [
        { text: 'hello', rawText: 'Hello', count: 3, lastUsed: 1000, firstSeen: 500, dismissedAt: null },
      ];
      const { saveInputStats } = await importModule();
      await saveInputStats(stats);
      expect(mockStore.set).toHaveBeenCalledWith('inputStats', stats);
      expect(mockStore.save).toHaveBeenCalled();
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test -- --run src/lib/services/persistenceService.inputStats.test.ts`
Expected: FAIL — `loadInputStats` and `saveInputStats` not found in module

- [ ] **Step 3: Implement the types and persistence functions**

Add to the bottom of `src/lib/services/persistenceService.ts`:

```typescript
// ============================================================================
// Input Stats (for shortcut suggestions)
// ============================================================================

export interface InputRecord {
  text: string;              // Normalized text (trim + lowercase)
  rawText: string;           // Original text (for shortcut label)
  count: number;             // Total usage count
  lastUsed: number;          // Last used timestamp (Unix ms)
  firstSeen: number;         // First seen timestamp
  dismissedAt: number | null; // Dismissal timestamp (for cooldown)
}

/**
 * Load input stats from settings
 */
export async function loadInputStats(): Promise<InputRecord[]> {
  try {
    const s = await getStore();
    await s.reload();
    const stats = await s.get<InputRecord[]>('inputStats');
    return stats ?? [];
  } catch (error) {
    console.error('Failed to load input stats:', error);
    return [];
  }
}

/**
 * Save input stats to settings
 */
export async function saveInputStats(stats: InputRecord[]): Promise<void> {
  try {
    const s = await getStore();
    await s.set('inputStats', stats);
    await s.save();
  } catch (error) {
    console.error('Failed to save input stats:', error);
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test -- --run src/lib/services/persistenceService.inputStats.test.ts`
Expected: All 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/services/persistenceService.ts src/lib/services/persistenceService.inputStats.test.ts
git commit -m "feat(shortcuts): add InputRecord type and persistence functions"
```

---

### Task 2: Create `inputStatsService` — core recording logic

**Files:**
- Create: `src/lib/services/inputStatsService.ts`
- Create: `src/lib/services/inputStatsService.test.ts`

- [ ] **Step 1: Write the failing tests for `normalizeText` and `record`**

Create `src/lib/services/inputStatsService.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock persistenceService
vi.mock('./persistenceService', () => ({
  loadInputStats: vi.fn().mockResolvedValue([]),
  saveInputStats: vi.fn().mockResolvedValue(undefined),
}));

import { createInputStatsService, normalizeText } from './inputStatsService';
import type { InputRecord } from './persistenceService';

describe('normalizeText', () => {
  it('should trim and lowercase text', () => {
    expect(normalizeText('  Hello World  ')).toBe('hello world');
  });

  it('should handle already normalized text', () => {
    expect(normalizeText('hello')).toBe('hello');
  });

  it('should preserve non-ASCII characters', () => {
    expect(normalizeText('  続けて  ')).toBe('続けて');
  });
});

describe('inputStatsService', () => {
  let service: ReturnType<typeof createInputStatsService>;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-03-27T00:00:00Z'));
    service = createInputStatsService();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('record', () => {
    it('should add a new record for unseen text', () => {
      service.record('Hello');
      const records = service.getRecords();
      expect(records).toHaveLength(1);
      expect(records[0]).toEqual({
        text: 'hello',
        rawText: 'Hello',
        count: 1,
        lastUsed: Date.now(),
        firstSeen: Date.now(),
        dismissedAt: null,
      });
    });

    it('should increment count for repeated text', () => {
      service.record('Hello');
      vi.advanceTimersByTime(1000);
      service.record('  hello  ');
      const records = service.getRecords();
      expect(records).toHaveLength(1);
      expect(records[0].count).toBe(2);
    });

    it('should update rawText to latest form', () => {
      service.record('Hello');
      service.record('HELLO');
      const records = service.getRecords();
      expect(records[0].rawText).toBe('HELLO');
    });

    it('should update lastUsed timestamp', () => {
      service.record('Hello');
      const first = service.getRecords()[0].lastUsed;
      vi.advanceTimersByTime(5000);
      service.record('Hello');
      expect(service.getRecords()[0].lastUsed).toBe(first + 5000);
    });

    it('should ignore empty or whitespace-only input', () => {
      service.record('');
      service.record('   ');
      service.record('\t\n');
      expect(service.getRecords()).toHaveLength(0);
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test -- --run src/lib/services/inputStatsService.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Implement `normalizeText` and core `createInputStatsService`**

Create `src/lib/services/inputStatsService.ts`:

```typescript
import type { InputRecord } from './persistenceService';

/**
 * Normalize input text for comparison: trim whitespace, lowercase.
 */
export function normalizeText(text: string): string {
  return text.trim().toLowerCase();
}

/**
 * Creates an input stats service instance.
 * Pure logic, no persistence — persistence is handled externally.
 */
export function createInputStatsService(initialRecords: InputRecord[] = []) {
  let records: InputRecord[] = [...initialRecords];

  return {
    /**
     * Record a user input. Ignores empty/whitespace-only text.
     */
    record(rawInput: string): void {
      const trimmed = rawInput.trim();
      if (trimmed.length === 0) return;

      const normalized = normalizeText(rawInput);
      const now = Date.now();
      const existing = records.find((r) => r.text === normalized);

      if (existing) {
        existing.count += 1;
        existing.lastUsed = now;
        existing.rawText = trimmed;
      } else {
        records.push({
          text: normalized,
          rawText: trimmed,
          count: 1,
          lastUsed: now,
          firstSeen: now,
          dismissedAt: null,
        });
      }
    },

    /**
     * Get all records (for testing and persistence).
     */
    getRecords(): InputRecord[] {
      return [...records];
    },

    /**
     * Replace all records (for loading from persistence).
     */
    setRecords(newRecords: InputRecord[]): void {
      records = [...newRecords];
    },
  };
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test -- --run src/lib/services/inputStatsService.test.ts`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/services/inputStatsService.ts src/lib/services/inputStatsService.test.ts
git commit -m "feat(shortcuts): add inputStatsService with core recording logic"
```

---

### Task 3: Add eviction logic (1000 entry limit)

**Files:**
- Modify: `src/lib/services/inputStatsService.ts`
- Modify: `src/lib/services/inputStatsService.test.ts`

- [ ] **Step 1: Write the failing test**

Add to `inputStatsService.test.ts` inside the `describe('inputStatsService')` block:

```typescript
  describe('eviction', () => {
    it('should evict lowest count + oldest when exceeding 1000 entries', () => {
      // Fill with 1000 records
      for (let i = 0; i < 1000; i++) {
        service.record(`text-${i}`);
      }
      expect(service.getRecords()).toHaveLength(1000);

      // Record one more — should evict the oldest with count=1
      vi.advanceTimersByTime(1000);
      service.record('new-entry');
      const records = service.getRecords();
      expect(records).toHaveLength(1000);

      // new-entry should exist
      expect(records.find((r) => r.text === 'new-entry')).toBeDefined();

      // text-0 should have been evicted (oldest with count=1)
      expect(records.find((r) => r.text === 'text-0')).toBeUndefined();
    });

    it('should evict by lowest count first, then oldest lastUsed', () => {
      const now = Date.now();
      const initialRecords: InputRecord[] = [];
      for (let i = 0; i < 1000; i++) {
        initialRecords.push({
          text: `text-${i}`,
          rawText: `text-${i}`,
          count: i < 500 ? 1 : 5,
          lastUsed: now - (1000 - i) * 1000,
          firstSeen: now - 100000,
          dismissedAt: null,
        });
      }
      service = createInputStatsService(initialRecords);

      vi.advanceTimersByTime(1000);
      service.record('new-entry');
      const records = service.getRecords();

      // text-0 has count=1 and oldest lastUsed, should be evicted
      expect(records.find((r) => r.text === 'text-0')).toBeUndefined();
      expect(records.find((r) => r.text === 'new-entry')).toBeDefined();
    });
  });
```

Also add this import at the top of the test file if not already there:

```typescript
import type { InputRecord } from './persistenceService';
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test -- --run src/lib/services/inputStatsService.test.ts`
Expected: FAIL — records exceed 1000

- [ ] **Step 3: Add eviction to `record` method**

In `src/lib/services/inputStatsService.ts`, add this constant at the top:

```typescript
const MAX_RECORDS = 1000;
```

Add this at the end of the `record` method, after adding the new record:

```typescript
      // Evict if over limit
      if (records.length > MAX_RECORDS) {
        // Sort by count ascending, then by lastUsed ascending (oldest first)
        let minIndex = 0;
        for (let i = 1; i < records.length; i++) {
          const min = records[minIndex];
          const curr = records[i];
          if (
            curr.count < min.count ||
            (curr.count === min.count && curr.lastUsed < min.lastUsed)
          ) {
            minIndex = i;
          }
        }
        records.splice(minIndex, 1);
      }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test -- --run src/lib/services/inputStatsService.test.ts`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/services/inputStatsService.ts src/lib/services/inputStatsService.test.ts
git commit -m "feat(shortcuts): add 1000-entry eviction to inputStatsService"
```

---

### Task 4: Add suggestion logic

**Files:**
- Modify: `src/lib/services/inputStatsService.ts`
- Modify: `src/lib/services/inputStatsService.test.ts`

- [ ] **Step 1: Write the failing tests**

Add to `inputStatsService.test.ts`:

```typescript
  describe('getSuggestions', () => {
    const existingShortcutTexts = ['ok', 'continue', 'lgtm'];
    const SEVEN_DAYS = 7 * 24 * 60 * 60 * 1000;

    it('should suggest text used 3+ times within 7 days', () => {
      service.record('deploy');
      service.record('deploy');
      service.record('deploy');
      const suggestions = service.getSuggestions(existingShortcutTexts);
      expect(suggestions).toHaveLength(1);
      expect(suggestions[0].rawText).toBe('deploy');
      expect(suggestions[0].count).toBe(3);
    });

    it('should not suggest text used less than 3 times', () => {
      service.record('deploy');
      service.record('deploy');
      expect(service.getSuggestions(existingShortcutTexts)).toHaveLength(0);
    });

    it('should not suggest text older than 7 days', () => {
      service.record('deploy');
      service.record('deploy');
      service.record('deploy');
      vi.advanceTimersByTime(SEVEN_DAYS + 1);
      expect(service.getSuggestions(existingShortcutTexts)).toHaveLength(0);
    });

    it('should not suggest text that matches an existing shortcut', () => {
      service.record('OK');
      service.record('OK');
      service.record('OK');
      expect(service.getSuggestions(existingShortcutTexts)).toHaveLength(0);
    });

    it('should not suggest dismissed text within cooldown period', () => {
      service.record('deploy');
      service.record('deploy');
      service.record('deploy');
      service.dismiss('deploy');
      expect(service.getSuggestions(existingShortcutTexts)).toHaveLength(0);
    });

    it('should re-suggest after cooldown period (7 days)', () => {
      service.record('deploy');
      service.record('deploy');
      service.record('deploy');
      service.dismiss('deploy');
      vi.advanceTimersByTime(SEVEN_DAYS + 1);
      // Need to record again to update lastUsed within 7 days
      service.record('deploy');
      expect(service.getSuggestions(existingShortcutTexts)).toHaveLength(1);
    });

    it('should return max 5 suggestions sorted by count descending', () => {
      for (let i = 0; i < 7; i++) {
        const text = `cmd-${i}`;
        for (let j = 0; j < 3 + i; j++) {
          service.record(text);
        }
      }
      const suggestions = service.getSuggestions(existingShortcutTexts);
      expect(suggestions).toHaveLength(5);
      // Highest count first
      expect(suggestions[0].count).toBeGreaterThanOrEqual(suggestions[1].count);
      expect(suggestions[4].count).toBe(5); // cmd-2 has count 5, the lowest in top 5
    });
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test -- --run src/lib/services/inputStatsService.test.ts`
Expected: FAIL — `getSuggestions` and `dismiss` not defined

- [ ] **Step 3: Implement `getSuggestions` and `dismiss`**

Add these constants at the top of `inputStatsService.ts`:

```typescript
const MIN_COUNT = 3;
const MAX_SUGGESTIONS = 5;
const RECENCY_WINDOW_MS = 7 * 24 * 60 * 60 * 1000; // 7 days
const COOLDOWN_MS = 7 * 24 * 60 * 60 * 1000; // 7 days
```

Add these methods to the returned object in `createInputStatsService`:

```typescript
    /**
     * Get suggestion candidates based on frequency, recency, and exclusions.
     * @param existingShortcutTexts - Lowercased text of all existing shortcuts
     */
    getSuggestions(existingShortcutTexts: string[]): InputRecord[] {
      const now = Date.now();
      const shortcutSet = new Set(existingShortcutTexts);

      return records
        .filter((r) => {
          if (r.count < MIN_COUNT) return false;
          if (now - r.lastUsed > RECENCY_WINDOW_MS) return false;
          if (shortcutSet.has(r.text)) return false;
          if (r.dismissedAt !== null && now - r.dismissedAt < COOLDOWN_MS) return false;
          return true;
        })
        .sort((a, b) => b.count - a.count)
        .slice(0, MAX_SUGGESTIONS);
    },

    /**
     * Dismiss a suggestion (start cooldown).
     */
    dismiss(normalizedText: string): void {
      const record = records.find((r) => r.text === normalizedText);
      if (record) {
        record.dismissedAt = Date.now();
      }
    },

    /**
     * Remove a suggestion from records (when added to shortcuts).
     */
    removeSuggestion(normalizedText: string): void {
      records = records.filter((r) => r.text !== normalizedText);
    },
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test -- --run src/lib/services/inputStatsService.test.ts`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/services/inputStatsService.ts src/lib/services/inputStatsService.test.ts
git commit -m "feat(shortcuts): add suggestion logic with frequency, recency, and cooldown"
```

---

### Task 5: Add shortcut type auto-detection

**Files:**
- Modify: `src/lib/services/inputStatsService.ts`
- Modify: `src/lib/services/inputStatsService.test.ts`

- [ ] **Step 1: Write the failing tests**

Add to `inputStatsService.test.ts`:

```typescript
describe('detectShortcutType', () => {
  it('should return "command" for text starting with /', () => {
    expect(detectShortcutType('/commit')).toBe('command');
    expect(detectShortcutType('/review-pr')).toBe('command');
  });

  it('should return "reply" for text not starting with /', () => {
    expect(detectShortcutType('continue')).toBe('reply');
    expect(detectShortcutType('LGTM')).toBe('reply');
    expect(detectShortcutType('テスト実行して')).toBe('reply');
  });
});
```

Update the import:

```typescript
import { createInputStatsService, normalizeText, detectShortcutType } from './inputStatsService';
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test -- --run src/lib/services/inputStatsService.test.ts`
Expected: FAIL — `detectShortcutType` not found

- [ ] **Step 3: Implement `detectShortcutType`**

Add to `src/lib/services/inputStatsService.ts`:

```typescript
import type { ShortcutType } from '@/lib/stores/shortcutStore.svelte';

/**
 * Detect whether input should become a 'command' or 'reply' shortcut.
 */
export function detectShortcutType(text: string): ShortcutType {
  return text.startsWith('/') ? 'command' : 'reply';
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test -- --run src/lib/services/inputStatsService.test.ts`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/services/inputStatsService.ts src/lib/services/inputStatsService.test.ts
git commit -m "feat(shortcuts): add shortcut type auto-detection for suggestions"
```

---

### Task 6: Create `ShortcutSuggestions.svelte` component

**Files:**
- Create: `src/lib/components/terminal/ShortcutSuggestions.svelte`
- Create: `src/lib/components/terminal/ShortcutSuggestions.browser.test.ts`

- [ ] **Step 1: Write the failing browser tests**

Create `src/lib/components/terminal/ShortcutSuggestions.browser.test.ts`:

```typescript
import { render, fireEvent, cleanup } from '@testing-library/svelte';
import { describe, it, expect, vi, afterEach } from 'vitest';
import ShortcutSuggestions from './ShortcutSuggestions.svelte';
import type { InputRecord } from '@/lib/services/persistenceService';

describe('ShortcutSuggestions', () => {
  afterEach(() => {
    cleanup();
  });

  const makeSuggestion = (text: string, count: number): InputRecord => ({
    text: text.toLowerCase(),
    rawText: text,
    count,
    lastUsed: Date.now(),
    firstSeen: Date.now() - 100000,
    dismissedAt: null,
  });

  it('should not render badge when there are no suggestions', () => {
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions: [], onAdd: vi.fn(), onDismiss: vi.fn() },
    });
    expect(container.querySelector('.suggestion-badge')).toBeNull();
  });

  it('should render badge with count when there are suggestions', () => {
    const suggestions = [makeSuggestion('deploy', 5), makeSuggestion('test', 3)];
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd: vi.fn(), onDismiss: vi.fn() },
    });
    const badge = container.querySelector('.suggestion-badge');
    expect(badge).toBeTruthy();
    expect(badge!.textContent).toContain('+2');
  });

  it('should toggle popover on badge click', async () => {
    const suggestions = [makeSuggestion('deploy', 5)];
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd: vi.fn(), onDismiss: vi.fn() },
    });

    const badge = container.querySelector('.suggestion-badge')!;
    expect(container.querySelector('.suggestion-popover')).toBeNull();

    await fireEvent.click(badge);
    expect(container.querySelector('.suggestion-popover')).toBeTruthy();

    await fireEvent.click(badge);
    expect(container.querySelector('.suggestion-popover')).toBeNull();
  });

  it('should display suggestion text and count in popover', async () => {
    const suggestions = [makeSuggestion('deploy', 5)];
    const { container, getByText } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd: vi.fn(), onDismiss: vi.fn() },
    });

    await fireEvent.click(container.querySelector('.suggestion-badge')!);
    expect(getByText('deploy')).toBeTruthy();
    expect(getByText('5')).toBeTruthy();
  });

  it('should call onAdd when add button is clicked', async () => {
    const onAdd = vi.fn();
    const suggestions = [makeSuggestion('deploy', 5)];
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd, onDismiss: vi.fn() },
    });

    await fireEvent.click(container.querySelector('.suggestion-badge')!);
    const addBtn = container.querySelector('.suggestion-add-btn')!;
    await fireEvent.click(addBtn);
    expect(onAdd).toHaveBeenCalledWith(suggestions[0]);
  });

  it('should call onDismiss when dismiss button is clicked', async () => {
    const onDismiss = vi.fn();
    const suggestions = [makeSuggestion('deploy', 5)];
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd: vi.fn(), onDismiss },
    });

    await fireEvent.click(container.querySelector('.suggestion-badge')!);
    const dismissBtn = container.querySelector('.suggestion-dismiss-btn')!;
    await fireEvent.click(dismissBtn);
    expect(onDismiss).toHaveBeenCalledWith(suggestions[0]);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test:browser -- --run src/lib/components/terminal/ShortcutSuggestions.browser.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Implement `ShortcutSuggestions.svelte`**

Create `src/lib/components/terminal/ShortcutSuggestions.svelte`:

```svelte
<script lang="ts">
  import type { InputRecord } from '@/lib/services/persistenceService';

  interface Props {
    suggestions: InputRecord[];
    onAdd: (suggestion: InputRecord) => void;
    onDismiss: (suggestion: InputRecord) => void;
  }

  let { suggestions, onAdd, onDismiss }: Props = $props();

  let showPopover = $state(false);

  function togglePopover() {
    showPopover = !showPopover;
  }

  function handleAdd(suggestion: InputRecord) {
    onAdd(suggestion);
  }

  function handleDismiss(suggestion: InputRecord) {
    onDismiss(suggestion);
  }
</script>

{#if suggestions.length > 0}
  <div class="suggestion-wrapper">
    <button
      class="suggestion-badge"
      onclick={togglePopover}
      title="Shortcut suggestions"
      aria-label="Shortcut suggestions"
    >
      +{suggestions.length}
    </button>

    {#if showPopover}
      <div class="suggestion-popover">
        <div class="popover-header">Suggestions</div>
        <div class="popover-list">
          {#each suggestions as suggestion (suggestion.text)}
            <div class="suggestion-item">
              <span class="suggestion-text">{suggestion.rawText}</span>
              <span class="suggestion-count">{suggestion.count}</span>
              <button
                class="suggestion-add-btn"
                onclick={() => handleAdd(suggestion)}
                title="Add to shortcuts"
                aria-label="Add {suggestion.rawText} to shortcuts"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                  <line x1="12" y1="5" x2="12" y2="19" />
                  <line x1="5" y1="12" x2="19" y2="12" />
                </svg>
              </button>
              <button
                class="suggestion-dismiss-btn"
                onclick={() => handleDismiss(suggestion)}
                title="Dismiss suggestion"
                aria-label="Dismiss {suggestion.rawText}"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .suggestion-wrapper {
    position: relative;
  }

  .suggestion-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2px 8px;
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    color: var(--accent-color);
    background: rgba(125, 211, 252, 0.12);
    border: 1px solid rgba(125, 211, 252, 0.3);
    border-radius: 10px;
    cursor: pointer;
    text-shadow: 0 0 8px rgba(125, 211, 252, 0.3);
    transition:
      background var(--transition-fast),
      border-color var(--transition-fast),
      box-shadow var(--transition-fast);
  }

  .suggestion-badge:hover {
    background: rgba(125, 211, 252, 0.2);
    border-color: rgba(125, 211, 252, 0.5);
    box-shadow: 0 0 12px rgba(125, 211, 252, 0.2);
  }

  .suggestion-popover {
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    min-width: 240px;
    background: var(--bg-glass, rgba(13, 17, 23, 0.92));
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid rgba(125, 211, 252, 0.2);
    border-radius: var(--radius-lg, 12px);
    padding: 8px 0;
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.4),
      0 0 16px rgba(125, 211, 252, 0.06);
    animation: popoverSlideIn 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    z-index: 100;
  }

  @keyframes popoverSlideIn {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .popover-header {
    padding: 4px 12px 8px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted, rgba(230, 237, 243, 0.4));
    border-bottom: 1px solid rgba(125, 211, 252, 0.1);
  }

  .popover-list {
    padding: 4px 0;
  }

  .suggestion-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    transition: background var(--transition-fast);
  }

  .suggestion-item:hover {
    background: rgba(125, 211, 252, 0.06);
  }

  .suggestion-text {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary, #e6edf3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .suggestion-count {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-muted, rgba(230, 237, 243, 0.4));
    min-width: 16px;
    text-align: right;
  }

  .suggestion-add-btn,
  .suggestion-dismiss-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border-radius: var(--radius-sm, 6px);
    cursor: pointer;
    transition:
      color var(--transition-fast),
      background var(--transition-fast);
  }

  .suggestion-add-btn {
    color: rgba(125, 211, 252, 0.5);
    background: transparent;
    border: 1px solid transparent;
  }

  .suggestion-add-btn:hover {
    color: var(--accent-color);
    background: rgba(125, 211, 252, 0.12);
    border-color: rgba(125, 211, 252, 0.3);
  }

  .suggestion-dismiss-btn {
    color: rgba(230, 237, 243, 0.3);
    background: transparent;
    border: 1px solid transparent;
  }

  .suggestion-dismiss-btn:hover {
    color: var(--text-secondary, rgba(230, 237, 243, 0.7));
    background: rgba(230, 237, 243, 0.06);
    border-color: rgba(230, 237, 243, 0.15);
  }

  .suggestion-add-btn:active,
  .suggestion-dismiss-btn:active {
    transform: scale(0.9);
  }
</style>
```

- [ ] **Step 4: Run browser tests to verify they pass**

Run: `npm run test:browser -- --run src/lib/components/terminal/ShortcutSuggestions.browser.test.ts`
Expected: All 6 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/terminal/ShortcutSuggestions.svelte src/lib/components/terminal/ShortcutSuggestions.browser.test.ts
git commit -m "feat(shortcuts): add ShortcutSuggestions badge and popover component"
```

---

### Task 7: Add suggestion badge to `TerminalShortcutBar`

**Files:**
- Modify: `src/lib/components/terminal/TerminalShortcutBar.svelte`

- [ ] **Step 1: Add suggestion badge props and rendering**

In `TerminalShortcutBar.svelte`, update the `Props` interface:

```typescript
  import type { InputRecord } from '@/lib/services/persistenceService';
  import ShortcutSuggestions from './ShortcutSuggestions.svelte';

  interface Props {
    visible: boolean;
    shortcuts: TerminalShortcut[];
    showNumberRow: boolean;
    suggestions: InputRecord[];
    onSend: (text: string, withEnter: boolean) => void;
    onSettingsClick: () => void;
    onAddClick: (type: ShortcutType) => void;
    onSuggestionAdd: (suggestion: InputRecord) => void;
    onSuggestionDismiss: (suggestion: InputRecord) => void;
  }

  let { visible, shortcuts, showNumberRow, suggestions, onSend, onSettingsClick, onAddClick, onSuggestionAdd, onSuggestionDismiss }: Props = $props();
```

Add the `ShortcutSuggestions` component next to the settings button, replacing the existing settings button section:

```svelte
    <!-- Settings and suggestions -->
    <div class="bar-actions">
      <ShortcutSuggestions
        {suggestions}
        onAdd={onSuggestionAdd}
        onDismiss={onSuggestionDismiss}
      />
      <button
        class="settings-btn"
        onclick={onSettingsClick}
        title="Shortcut Settings"
        aria-label="Shortcut Settings"
      >
        <!-- existing SVG icon -->
      </button>
    </div>
```

Move the settings button from `position: absolute` to inside `bar-actions` flex container. Add styles:

```css
  .bar-actions {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .settings-btn {
    /* Remove position: absolute, top, right — now inside .bar-actions */
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    color: var(--text-muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition:
      color var(--transition-fast),
      background var(--transition-fast),
      border-color var(--transition-fast);
  }
```

- [ ] **Step 2: Update existing browser tests for TerminalShortcutBar**

Update `defaultProps` in `TerminalShortcutBar.browser.test.ts` to include the new props:

```typescript
  const defaultProps = {
    visible: true,
    shortcuts: [
      { id: 'builtin-ok', label: 'OK', text: 'OK', builtin: true, type: 'reply' as const },
      { id: 'builtin-continue', label: 'Continue', text: 'continue', builtin: true, type: 'reply' as const },
      { id: 'builtin-lgtm', label: 'LGTM', text: 'LGTM', builtin: true, type: 'reply' as const },
    ],
    suggestions: [],
    onSend: vi.fn(),
    onSettingsClick: vi.fn(),
    onAddClick: vi.fn(),
    onSuggestionAdd: vi.fn(),
    onSuggestionDismiss: vi.fn(),
  };
```

- [ ] **Step 3: Run all tests**

Run: `npm run test:browser -- --run src/lib/components/terminal/TerminalShortcutBar.browser.test.ts`
Expected: All existing tests PASS with updated props

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/terminal/TerminalShortcutBar.svelte src/lib/components/terminal/TerminalShortcutBar.browser.test.ts
git commit -m "feat(shortcuts): integrate suggestion badge into shortcut bar"
```

---

### Task 8: Wire everything together in `Terminal.svelte`

**Files:**
- Modify: `src/lib/components/terminal/Terminal.svelte`

- [ ] **Step 1: Add imports and state**

Add imports at the top of the `<script>` section:

```typescript
import { loadInputStats, saveInputStats, type InputRecord } from '@/lib/services/persistenceService';
import { createInputStatsService, normalizeText, detectShortcutType } from '@/lib/services/inputStatsService';
import { toastStore } from '@/lib/stores/toastStore';
```

Add state variables:

```typescript
  // Input stats for shortcut suggestions
  const inputStats = createInputStatsService();
  let suggestions = $state<InputRecord[]>([]);
  let inputBuffer = '';
  let saveDebounceTimer: ReturnType<typeof setTimeout> | null = null;
```

- [ ] **Step 2: Add input capture in `onData` handler**

Modify the existing `onData` handler at line ~605. Add input buffering logic before the `writeTerminal` call:

```typescript
      terminal.onData((data) => {
        if (terminalId !== null) {
          // Capture input for shortcut suggestions (only when AI is running)
          if (isAiRunning) {
            if (data === '\r') {
              // Enter pressed — record the buffered input
              if (inputBuffer.trim().length > 0) {
                inputStats.record(inputBuffer);
                updateSuggestions();
                scheduleSave();
              }
              inputBuffer = '';
            } else if (data.length === 1 && data.charCodeAt(0) >= 32) {
              // Regular printable character
              inputBuffer += data;
            } else if (data === '\x7f') {
              // Backspace
              inputBuffer = inputBuffer.slice(0, -1);
            } else if (data.length > 1 && !data.startsWith('\x1b')) {
              // Pasted text (multi-char, not escape sequence)
              inputBuffer += data;
            }
          }

          terminalService.writeTerminal(terminalId, data);
        }
      });
```

- [ ] **Step 3: Add helper functions**

Add these functions in the `<script>` section:

```typescript
  function updateSuggestions() {
    const existingTexts = shortcutState.allShortcuts.map((s) => s.text.trim().toLowerCase());
    suggestions = inputStats.getSuggestions(existingTexts);
  }

  function scheduleSave() {
    if (saveDebounceTimer) clearTimeout(saveDebounceTimer);
    saveDebounceTimer = setTimeout(async () => {
      await saveInputStats(inputStats.getRecords());
      saveDebounceTimer = null;
    }, 5000);
  }

  async function handleSuggestionAdd(suggestion: InputRecord) {
    const type = detectShortcutType(suggestion.rawText);
    await handleShortcutAdd(suggestion.rawText, suggestion.rawText, type);
    inputStats.removeSuggestion(suggestion.text);
    updateSuggestions();
    scheduleSave();
    toastStore.success('Added to shortcuts');
  }

  function handleSuggestionDismiss(suggestion: InputRecord) {
    inputStats.dismiss(suggestion.text);
    updateSuggestions();
    scheduleSave();
  }
```

- [ ] **Step 4: Load stats on mount**

In the `initTerminal` function (or the `onMount`), add loading of persisted stats:

```typescript
    // Load input stats for shortcut suggestions
    const savedStats = await loadInputStats();
    inputStats.setRecords(savedStats);
    updateSuggestions();
```

- [ ] **Step 5: Save on unmount**

In the existing `onDestroy` or return function from `onMount`, add:

```typescript
    // Save any pending input stats
    if (saveDebounceTimer) {
      clearTimeout(saveDebounceTimer);
    }
    await saveInputStats(inputStats.getRecords());
```

- [ ] **Step 6: Pass suggestions to `TerminalShortcutBar`**

Update the `TerminalShortcutBar` component in the template:

```svelte
  <TerminalShortcutBar
    visible={isAiRunning}
    shortcuts={shortcutState.allShortcuts}
    showNumberRow={numberRowEnabled}
    {suggestions}
    onSend={handleShortcutSend}
    onSettingsClick={() => {
      shortcutFocusSection = null;
      showShortcutSettings = true;
    }}
    onAddClick={handleShortcutAddClick}
    onSuggestionAdd={handleSuggestionAdd}
    onSuggestionDismiss={handleSuggestionDismiss}
  />
```

- [ ] **Step 7: Run all tests**

Run: `npm run test && npm run test:browser`
Expected: All tests PASS

- [ ] **Step 8: Run lint and type check**

Run: `npm run lint && npm run check`
Expected: No errors

- [ ] **Step 9: Commit**

```bash
git add src/lib/components/terminal/Terminal.svelte
git commit -m "feat(shortcuts): wire input capture and suggestion system into terminal"
```

---

### Task 9: Manual verification

**Files:** None (testing only)

- [ ] **Step 1: Start the app**

Run: `npm run tauri dev`

- [ ] **Step 2: Verify shortcut suggestions**

1. Start Claude in the terminal
2. Type the same reply 3+ times (e.g., "continue")
3. Verify the suggestion badge appears on the shortcut bar
4. Click the badge — verify popover shows the suggestion with count
5. Click `[+]` — verify shortcut is added, toast appears
6. Verify the suggestion disappears from the badge

- [ ] **Step 3: Verify dismissal**

1. Type a new phrase 3+ times
2. Click the dismiss `[×]` button
3. Verify the suggestion disappears
4. Type the same phrase again — verify it does not reappear immediately

- [ ] **Step 4: Verify existing shortcuts are excluded**

1. Type "OK" (a built-in shortcut) 3+ times
2. Verify it does NOT appear as a suggestion

- [ ] **Step 5: Take screenshot for verification**

Use `webview_screenshot` to capture the shortcut bar with suggestion badge visible.
