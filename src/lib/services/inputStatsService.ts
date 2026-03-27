import type { InputRecord } from './persistenceService';

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
