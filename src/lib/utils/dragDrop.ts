/**
 * Get the parent directory path from a file/directory path.
 */
export function getParentDirectory(filePath: string): string {
  const normalized = filePath.endsWith('/') ? filePath.slice(0, -1) : filePath;
  const lastSlash = normalized.lastIndexOf('/');
  if (lastSlash <= 0) return '/';
  return normalized.slice(0, lastSlash);
}
