import type { InputRecord } from './persistenceService';

// ============================================================================
// Constants
// ============================================================================

export const MAX_RECORDS = 1000;

const MIN_COUNT = 3;
const MAX_SUGGESTIONS = 5;
const RECENCY_WINDOW_MS = 7 * 24 * 60 * 60 * 1000; // 7 days
const COOLDOWN_MS = 7 * 24 * 60 * 60 * 1000; // 7 days

// ============================================================================
// normalizeText
// ============================================================================

/**
 * Normalize input text: trim whitespace and convert to lowercase.
 */
export function normalizeText(text: string): string {
  return text.trim().toLowerCase();
}

// ============================================================================
// createInputStatsService
// ============================================================================

export interface InputStatsService {
  record(rawInput: string): void;
  getRecords(): InputRecord[];
  setRecords(newRecords: InputRecord[]): void;
  getSuggestions(existingShortcutTexts: string[]): InputRecord[];
  dismiss(normalizedText: string): void;
  removeSuggestion(normalizedText: string): void;
}

/**
 * Factory function that creates an input stats service for tracking and suggesting shortcuts.
 */
export function createInputStatsService(initialRecords?: InputRecord[]): InputStatsService {
  let records: InputRecord[] = initialRecords ? [...initialRecords] : [];

  return {
    record(rawInput: string): void {
      const normalized = normalizeText(rawInput);
      if (!normalized) return;

      const now = Date.now();
      const existing = records.find((r) => r.text === normalized);

      if (existing) {
        existing.count += 1;
        existing.rawText = rawInput;
        existing.lastUsed = now;
      } else {
        records.push({
          text: normalized,
          rawText: rawInput,
          count: 1,
          lastUsed: now,
          firstSeen: now,
          dismissedAt: null,
        });

        // Evict if over the limit
        if (records.length > MAX_RECORDS) {
          // Find the entry with the lowest count; ties broken by oldest lastUsed
          let evictIndex = 0;
          for (let i = 1; i < records.length; i++) {
            const candidate = records[i];
            const current = records[evictIndex];
            if (
              candidate.count < current.count ||
              (candidate.count === current.count && candidate.lastUsed < current.lastUsed)
            ) {
              evictIndex = i;
            }
          }
          records.splice(evictIndex, 1);
        }
      }
    },

    getRecords(): InputRecord[] {
      return [...records];
    },

    setRecords(newRecords: InputRecord[]): void {
      records = [...newRecords];
    },

    getSuggestions(existingShortcutTexts: string[]): InputRecord[] {
      const now = Date.now();
      const existingSet = new Set(existingShortcutTexts.map((t) => t.toLowerCase()));

      return records
        .filter((r) => {
          // Must have at least MIN_COUNT uses
          if (r.count < MIN_COUNT) return false;
          // Must have been used within the recency window
          if (now - r.lastUsed > RECENCY_WINDOW_MS) return false;
          // Must not match an existing shortcut
          if (existingSet.has(r.text)) return false;
          // Must not be in cooldown
          if (r.dismissedAt !== null && now - r.dismissedAt < COOLDOWN_MS) return false;
          return true;
        })
        .sort((a, b) => b.count - a.count)
        .slice(0, MAX_SUGGESTIONS);
    },

    dismiss(normalizedText: string): void {
      const record = records.find((r) => r.text === normalizedText);
      if (record) {
        record.dismissedAt = Date.now();
      }
    },

    removeSuggestion(normalizedText: string): void {
      records = records.filter((r) => r.text !== normalizedText);
    },
  };
}
