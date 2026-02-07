/**
 * Format bytes into a human-readable string
 * e.g., 0 → "0 MB", 500000 → "< 1 MB", 44040192 → "42 MB", 1288490188 → "1.2 GB"
 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 MB';

  const mb = bytes / (1024 * 1024);
  if (mb < 1) return '< 1 MB';

  const gb = mb / 1024;
  if (gb >= 1) {
    return `${gb.toFixed(1)} GB`;
  }

  return `${Math.round(mb)} MB`;
}
