import type { InputRecord } from './persistenceService';

// ============================================================================
// Constants
// ============================================================================

export const MAX_RECORDS = 1000;

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
  };
}
