import { describe, it, expect } from 'vitest';
import { formatRelativeTime, formatDate } from './dateFormat';

describe('formatRelativeTime', () => {
  // Use a fixed "now" time for consistent tests: 2026-01-15 12:00:00 UTC
  const now = Date.UTC(2026, 0, 15, 12, 0, 0); // 1736942400000 ms

  it('returns "just now" for timestamps less than 1 minute ago', () => {
    const timestamp = Math.floor(now / 1000) - 30; // 30 seconds ago
    expect(formatRelativeTime(timestamp, now)).toBe('just now');
  });

  it('returns "1min ago" for exactly 1 minute ago', () => {
    const timestamp = Math.floor(now / 1000) - 60;
    expect(formatRelativeTime(timestamp, now)).toBe('1min ago');
  });

  it('returns "Xmins ago" for minutes', () => {
    const timestamp = Math.floor(now / 1000) - 5 * 60; // 5 minutes ago
    expect(formatRelativeTime(timestamp, now)).toBe('5mins ago');
  });

  it('returns "1hour ago" for exactly 1 hour ago', () => {
    const timestamp = Math.floor(now / 1000) - 60 * 60;
    expect(formatRelativeTime(timestamp, now)).toBe('1hour ago');
  });

  it('returns "Xhours ago" for hours', () => {
    const timestamp = Math.floor(now / 1000) - 3 * 60 * 60; // 3 hours ago
    expect(formatRelativeTime(timestamp, now)).toBe('3hours ago');
  });

  it('returns "20hours ago" for 20 hours ago', () => {
    const timestamp = Math.floor(now / 1000) - 20 * 60 * 60;
    expect(formatRelativeTime(timestamp, now)).toBe('20hours ago');
  });

  it('returns "1day ago" for exactly 1 day ago', () => {
    const timestamp = Math.floor(now / 1000) - 24 * 60 * 60;
    expect(formatRelativeTime(timestamp, now)).toBe('1day ago');
  });

  it('returns "Xdays ago" for days', () => {
    const timestamp = Math.floor(now / 1000) - 3 * 24 * 60 * 60; // 3 days ago
    expect(formatRelativeTime(timestamp, now)).toBe('3days ago');
  });

  it('returns "1week ago" for exactly 1 week ago', () => {
    const timestamp = Math.floor(now / 1000) - 7 * 24 * 60 * 60;
    expect(formatRelativeTime(timestamp, now)).toBe('1week ago');
  });

  it('returns "Xweeks ago" for weeks', () => {
    const timestamp = Math.floor(now / 1000) - 3 * 7 * 24 * 60 * 60; // 3 weeks ago
    expect(formatRelativeTime(timestamp, now)).toBe('3weeks ago');
  });

  it('returns date format for timestamps older than 4 weeks', () => {
    const timestamp = Math.floor(now / 1000) - 5 * 7 * 24 * 60 * 60; // 5 weeks ago
    expect(formatRelativeTime(timestamp, now)).toBe('11 Dec 2025');
  });

  it('returns date format for future timestamps', () => {
    const timestamp = Math.floor(now / 1000) + 60 * 60; // 1 hour in future
    expect(formatRelativeTime(timestamp, now)).toBe('15 Jan 2026');
  });
});

describe('formatDate', () => {
  it('formats a timestamp as "DD Mon YYYY"', () => {
    // 2026-01-06 12:00:00 UTC
    const timestamp = Math.floor(Date.UTC(2026, 0, 6, 12, 0, 0) / 1000);
    expect(formatDate(timestamp)).toBe('06 Jan 2026');
  });

  it('pads single-digit days with zero', () => {
    // 2026-03-05
    const timestamp = Math.floor(Date.UTC(2026, 2, 5, 12, 0, 0) / 1000);
    expect(formatDate(timestamp)).toBe('05 Mar 2026');
  });

  it('handles all months correctly', () => {
    const months = [
      'Jan',
      'Feb',
      'Mar',
      'Apr',
      'May',
      'Jun',
      'Jul',
      'Aug',
      'Sep',
      'Oct',
      'Nov',
      'Dec',
    ];
    months.forEach((month, index) => {
      const timestamp = Math.floor(Date.UTC(2026, index, 15, 12, 0, 0) / 1000);
      expect(formatDate(timestamp)).toBe(`15 ${month} 2026`);
    });
  });
});
