import { describe, it, expect } from 'vitest';
import { formatBytes } from './formatBytes';

describe('formatBytes', () => {
  it('should return "0 MB" for 0 bytes', () => {
    expect(formatBytes(0)).toBe('0 MB');
  });

  it('should return "0 MB" for negative bytes', () => {
    expect(formatBytes(-100)).toBe('0 MB');
  });

  it('should return "< 1 MB" for small values', () => {
    expect(formatBytes(1)).toBe('< 1 MB');
    expect(formatBytes(500_000)).toBe('< 1 MB');
    expect(formatBytes(1_048_575)).toBe('< 1 MB');
  });

  it('should return MB for values >= 1 MB', () => {
    expect(formatBytes(1_048_576)).toBe('1 MB');
    expect(formatBytes(10_485_760)).toBe('10 MB');
    expect(formatBytes(44_040_192)).toBe('42 MB');
    expect(formatBytes(104_857_600)).toBe('100 MB');
  });

  it('should round MB to nearest integer', () => {
    // 1.5 MB = 1,572,864 bytes
    expect(formatBytes(1_572_864)).toBe('2 MB');
    // 1.4 MB = 1,468,006 bytes
    expect(formatBytes(1_468_006)).toBe('1 MB');
  });

  it('should return GB for values >= 1 GB', () => {
    // 1 GB = 1,073,741,824 bytes
    expect(formatBytes(1_073_741_824)).toBe('1.0 GB');
    // 1.2 GB = 1,288,490,189 bytes
    expect(formatBytes(1_288_490_189)).toBe('1.2 GB');
    // 2.5 GB
    expect(formatBytes(2_684_354_560)).toBe('2.5 GB');
  });
});
