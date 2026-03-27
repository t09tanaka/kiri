import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createInputStatsService, MAX_RECORDS, normalizeText } from './inputStatsService';

// ============================================================================
// normalizeText
// ============================================================================

describe('normalizeText', () => {
  it('trims whitespace and lowercases', () => {
    expect(normalizeText('  Hello World  ')).toBe('hello world');
  });

  it('returns already-normalized text unchanged', () => {
    expect(normalizeText('hello world')).toBe('hello world');
  });

  it('handles non-ASCII characters', () => {
    expect(normalizeText('  テスト  ')).toBe('テスト');
  });

  it('handles empty string', () => {
    expect(normalizeText('')).toBe('');
  });
});

// ============================================================================
// record
// ============================================================================

describe('createInputStatsService - record', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('records a new text entry with count=1', () => {
    const now = 1000000;
    vi.setSystemTime(now);
    const service = createInputStatsService();
    service.record('hello');
    const records = service.getRecords();
    expect(records).toHaveLength(1);
    expect(records[0].text).toBe('hello');
    expect(records[0].rawText).toBe('hello');
    expect(records[0].count).toBe(1);
    expect(records[0].lastUsed).toBe(now);
    expect(records[0].firstSeen).toBe(now);
    expect(records[0].dismissedAt).toBeNull();
  });

  it('increments count when same text is recorded again', () => {
    const service = createInputStatsService();
    service.record('hello');
    service.record('hello');
    const records = service.getRecords();
    expect(records).toHaveLength(1);
    expect(records[0].count).toBe(2);
  });

  it('normalizes text for deduplication (case-insensitive)', () => {
    const service = createInputStatsService();
    service.record('Hello');
    service.record('HELLO');
    const records = service.getRecords();
    expect(records).toHaveLength(1);
    expect(records[0].count).toBe(2);
  });

  it('updates rawText to latest form on repeated record', () => {
    const service = createInputStatsService();
    service.record('hello');
    service.record('Hello');
    const records = service.getRecords();
    expect(records[0].rawText).toBe('Hello');
  });

  it('updates lastUsed on repeated record', () => {
    vi.setSystemTime(1000);
    const service = createInputStatsService();
    service.record('hello');
    vi.setSystemTime(2000);
    service.record('hello');
    const records = service.getRecords();
    expect(records[0].lastUsed).toBe(2000);
  });

  it('ignores empty string', () => {
    const service = createInputStatsService();
    service.record('');
    expect(service.getRecords()).toHaveLength(0);
  });

  it('ignores whitespace-only string', () => {
    const service = createInputStatsService();
    service.record('   ');
    expect(service.getRecords()).toHaveLength(0);
  });

  it('trims and normalizes before recording', () => {
    const service = createInputStatsService();
    service.record('  Hello  ');
    const records = service.getRecords();
    expect(records[0].text).toBe('hello');
    expect(records[0].rawText).toBe('  Hello  ');
  });
});

// ============================================================================
// getRecords / setRecords
// ============================================================================

describe('createInputStatsService - getRecords / setRecords', () => {
  it('getRecords returns a copy, not the internal array', () => {
    const service = createInputStatsService();
    service.record('hello');
    const records1 = service.getRecords();
    records1.push({
      text: 'injected',
      rawText: 'injected',
      count: 99,
      lastUsed: 0,
      firstSeen: 0,
      dismissedAt: null,
    });
    expect(service.getRecords()).toHaveLength(1);
  });

  it('setRecords replaces internal records', () => {
    const service = createInputStatsService();
    service.record('hello');
    service.setRecords([
      {
        text: 'replaced',
        rawText: 'Replaced',
        count: 5,
        lastUsed: 100,
        firstSeen: 50,
        dismissedAt: null,
      },
    ]);
    const records = service.getRecords();
    expect(records).toHaveLength(1);
    expect(records[0].text).toBe('replaced');
  });

  it('accepts initialRecords in factory', () => {
    const initial = [
      {
        text: 'init',
        rawText: 'init',
        count: 3,
        lastUsed: 500,
        firstSeen: 100,
        dismissedAt: null,
      },
    ];
    const service = createInputStatsService(initial);
    expect(service.getRecords()).toHaveLength(1);
    expect(service.getRecords()[0].count).toBe(3);
  });
});

// ============================================================================
// Eviction (Task 3)
// ============================================================================

describe('createInputStatsService - eviction', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('MAX_RECORDS is 1000', () => {
    expect(MAX_RECORDS).toBe(1000);
  });

  it('evicts when exceeding MAX_RECORDS: new entry exists, oldest-lowest removed', () => {
    vi.setSystemTime(1);
    const initial = Array.from({ length: MAX_RECORDS }, (_, i) => ({
      text: `entry-${i}`,
      rawText: `entry-${i}`,
      count: 1,
      lastUsed: 1,
      firstSeen: 1,
      dismissedAt: null,
    }));
    const service = createInputStatsService(initial);
    vi.setSystemTime(2);
    service.record('new-entry');
    const records = service.getRecords();
    expect(records).toHaveLength(MAX_RECORDS);
    expect(records.some((r) => r.text === 'new-entry')).toBe(true);
    expect(records.some((r) => r.text === 'entry-0')).toBe(false);
  });

  it('evicts by lowest count first (not just oldest)', () => {
    const initial = Array.from({ length: MAX_RECORDS }, (_, i) => ({
      text: `entry-${i}`,
      rawText: `entry-${i}`,
      count: i === 0 ? 1 : 2,
      lastUsed: i + 1,
      firstSeen: 1,
      dismissedAt: null,
    }));
    const service = createInputStatsService(initial);
    service.record('new-entry');
    const records = service.getRecords();
    expect(records.some((r) => r.text === 'entry-0')).toBe(false);
    expect(records.some((r) => r.text === 'new-entry')).toBe(true);
  });

  it('evicts by oldest lastUsed when counts are tied', () => {
    const initial = Array.from({ length: MAX_RECORDS }, (_, i) => ({
      text: `entry-${i}`,
      rawText: `entry-${i}`,
      count: 1,
      lastUsed: i + 1,
      firstSeen: 1,
      dismissedAt: null,
    }));
    const service = createInputStatsService(initial);
    service.record('new-entry');
    const records = service.getRecords();
    expect(records.some((r) => r.text === 'entry-0')).toBe(false);
    expect(records.some((r) => r.text === 'new-entry')).toBe(true);
  });
});

// ============================================================================
// getSuggestions (Task 4)
// ============================================================================

describe('createInputStatsService - getSuggestions', () => {
  const SEVEN_DAYS_MS = 7 * 24 * 60 * 60 * 1000;
  const NOW = 1_000_000_000_000;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(NOW);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('suggests entries with count >= 3 within 7 days', () => {
    const service = createInputStatsService([
      {
        text: 'hello',
        rawText: 'hello',
        count: 3,
        lastUsed: NOW,
        firstSeen: NOW,
        dismissedAt: null,
      },
    ]);
    const suggestions = service.getSuggestions([]);
    expect(suggestions).toHaveLength(1);
    expect(suggestions[0].text).toBe('hello');
  });

  it('does NOT suggest entries with count < 3', () => {
    const service = createInputStatsService([
      {
        text: 'hello',
        rawText: 'hello',
        count: 2,
        lastUsed: NOW,
        firstSeen: NOW,
        dismissedAt: null,
      },
    ]);
    expect(service.getSuggestions([])).toHaveLength(0);
  });

  it('does NOT suggest entries older than 7 days', () => {
    const service = createInputStatsService([
      {
        text: 'hello',
        rawText: 'hello',
        count: 5,
        lastUsed: NOW - SEVEN_DAYS_MS - 1,
        firstSeen: NOW - SEVEN_DAYS_MS - 1,
        dismissedAt: null,
      },
    ]);
    expect(service.getSuggestions([])).toHaveLength(0);
  });

  it('does NOT suggest if text matches existing shortcut (case-insensitive)', () => {
    const service = createInputStatsService([
      {
        text: 'continue',
        rawText: 'continue',
        count: 5,
        lastUsed: NOW,
        firstSeen: NOW,
        dismissedAt: null,
      },
    ]);
    expect(service.getSuggestions(['Continue'])).toHaveLength(0);
  });

  it('does NOT suggest during cooldown (dismissed within 7 days)', () => {
    const service = createInputStatsService([
      {
        text: 'hello',
        rawText: 'hello',
        count: 5,
        lastUsed: NOW,
        firstSeen: NOW,
        dismissedAt: NOW - SEVEN_DAYS_MS + 1000,
      },
    ]);
    expect(service.getSuggestions([])).toHaveLength(0);
  });

  it('re-suggests after cooldown expires (dismissed > 7 days ago)', () => {
    const service = createInputStatsService([
      {
        text: 'hello',
        rawText: 'hello',
        count: 5,
        lastUsed: NOW,
        firstSeen: NOW,
        dismissedAt: NOW - SEVEN_DAYS_MS - 1,
      },
    ]);
    expect(service.getSuggestions([])).toHaveLength(1);
  });

  it('returns max 5 suggestions sorted by count desc', () => {
    const records = Array.from({ length: 8 }, (_, i) => ({
      text: `entry-${i}`,
      rawText: `entry-${i}`,
      count: i + 3,
      lastUsed: NOW,
      firstSeen: NOW,
      dismissedAt: null,
    }));
    const service = createInputStatsService(records);
    const suggestions = service.getSuggestions([]);
    expect(suggestions).toHaveLength(5);
    expect(suggestions[0].count).toBeGreaterThanOrEqual(suggestions[1].count);
    expect(suggestions[1].count).toBeGreaterThanOrEqual(suggestions[2].count);
    expect(suggestions[2].count).toBeGreaterThanOrEqual(suggestions[3].count);
    expect(suggestions[3].count).toBeGreaterThanOrEqual(suggestions[4].count);
  });
});

// ============================================================================
// dismiss / removeSuggestion (Task 4)
// ============================================================================

describe('createInputStatsService - dismiss / removeSuggestion', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('dismiss sets dismissedAt to Date.now()', () => {
    const now = 9999999;
    vi.setSystemTime(now);
    const service = createInputStatsService([
      {
        text: 'hello',
        rawText: 'hello',
        count: 5,
        lastUsed: now,
        firstSeen: now,
        dismissedAt: null,
      },
    ]);
    service.dismiss('hello');
    const records = service.getRecords();
    expect(records[0].dismissedAt).toBe(now);
  });

  it('removeSuggestion removes the record entirely', () => {
    const service = createInputStatsService([
      {
        text: 'hello',
        rawText: 'hello',
        count: 5,
        lastUsed: 1000,
        firstSeen: 1000,
        dismissedAt: null,
      },
    ]);
    service.removeSuggestion('hello');
    expect(service.getRecords()).toHaveLength(0);
  });
});
