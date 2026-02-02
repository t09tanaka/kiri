/**
 * Format a Unix timestamp to a relative time string.
 *
 * Examples:
 * - 1min ago
 * - 5mins ago
 * - 1hour ago
 * - 3hours ago
 * - 20hours ago
 * - 1day ago
 * - 3days ago
 * - 1week ago
 * - 3weeks ago
 * - 06 Jan 2026 (for dates older than 4 weeks)
 *
 * @param timestamp - Unix timestamp in seconds
 * @param now - Current time in milliseconds (optional, for testing)
 * @returns Formatted relative time string
 */
export function formatRelativeTime(timestamp: number, now?: number): string {
  const currentMs = now ?? Date.now();
  const diffSeconds = Math.floor((currentMs - timestamp * 1000) / 1000);

  if (diffSeconds < 0) {
    // Future time, just show the date
    return formatDate(timestamp);
  }

  const diffMinutes = Math.floor(diffSeconds / 60);
  const diffHours = Math.floor(diffMinutes / 60);
  const diffDays = Math.floor(diffHours / 24);
  const diffWeeks = Math.floor(diffDays / 7);

  if (diffMinutes < 1) {
    return 'just now';
  }

  if (diffMinutes < 60) {
    return diffMinutes === 1 ? '1min ago' : `${diffMinutes}mins ago`;
  }

  if (diffHours < 24) {
    return diffHours === 1 ? '1hour ago' : `${diffHours}hours ago`;
  }

  if (diffDays < 7) {
    return diffDays === 1 ? '1day ago' : `${diffDays}days ago`;
  }

  if (diffWeeks <= 4) {
    return diffWeeks === 1 ? '1week ago' : `${diffWeeks}weeks ago`;
  }

  // Older than 4 weeks, show the date
  return formatDate(timestamp);
}

/**
 * Format a Unix timestamp to a date string.
 *
 * @param timestamp - Unix timestamp in seconds
 * @returns Formatted date string (e.g., "06 Jan 2026")
 */
export function formatDate(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  const day = date.getDate().toString().padStart(2, '0');
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
  const month = months[date.getMonth()];
  const year = date.getFullYear();

  return `${day} ${month} ${year}`;
}
