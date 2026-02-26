/**
 * Get the parent directory path from a file/directory path.
 */
export function getParentDirectory(filePath: string): string {
  const normalized = filePath.endsWith('/') ? filePath.slice(0, -1) : filePath;
  const lastSlash = normalized.lastIndexOf('/');
  if (lastSlash <= 0) return '/';
  return normalized.slice(0, lastSlash);
}

/**
 * Resolve the drop target directory from the hovered element's data attributes.
 * - If hovering over a directory, returns that directory path.
 * - If hovering over a file, returns its parent directory.
 * - If no element, returns null (caller falls back to rootPath).
 */
export function resolveDropTarget(
  path: string | null,
  isDir: boolean,
  rootPath: string
): string | null {
  if (path === null) return null;
  if (isDir) return path;
  const parent = getParentDirectory(path);
  return parent.startsWith(rootPath) ? parent : rootPath;
}
