import { getFileStem, getTestFileBase, isConfigFile, isMarkdownFile } from '@/lib/utils/fileIcons';
import type { FileEntry } from './types';

/**
 * Sort order for file tree entries:
 *   1. Directories alphabetically
 *   2. Markdown files (grouped with their test peers)
 *   3. Regular source files (grouped with their test peers)
 *   4. Config files
 *
 * Within a group, the parent file comes first followed by its tests
 * alphabetically. Test peers are matched by stem so cross-extension
 * matches like `App.vue` ↔ `App.spec.ts` are colocated.
 */
export function sortFileEntries(items: FileEntry[]): FileEntry[] {
  return [...items].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    if (a.is_dir) {
      return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
    }
    const aBase = getTestFileBase(a.name);
    const bBase = getTestFileBase(b.name);
    const aGroupKey = getFileStem(aBase || a.name).toLowerCase();
    const bGroupKey = getFileStem(bBase || b.name).toLowerCase();
    const aIsMd = isMarkdownFile(aBase || a.name);
    const bIsMd = isMarkdownFile(bBase || b.name);
    if (aIsMd !== bIsMd) return aIsMd ? -1 : 1;
    const aIsConfig = isConfigFile(aBase || a.name);
    const bIsConfig = isConfigFile(bBase || b.name);
    if (aIsConfig !== bIsConfig) return aIsConfig ? 1 : -1;
    if (aGroupKey !== bGroupKey) {
      return aGroupKey.localeCompare(bGroupKey);
    }
    if (!aBase && bBase) return -1;
    if (aBase && !bBase) return 1;
    return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
  });
}

/**
 * Build placeholder `FileEntry` rows for files currently being dragged
 * into `targetDir`. They render in the tree before the copy finishes so
 * the UI doesn't flicker waiting for the watcher to refresh.
 */
export function buildPreviewEntries(draggedPaths: string[], targetDir: string): FileEntry[] {
  return draggedPaths.map((sourcePath) => {
    const name = sourcePath.split('/').pop() || sourcePath;
    return {
      name,
      path: `${targetDir}/${name}`,
      is_dir: false,
      is_hidden: name.startsWith('.'),
      is_gitignored: false,
      is_pending: true,
    } satisfies FileEntry;
  });
}

/**
 * Merge real entries with preview placeholders, deduping by path so a
 * preview never collides with an already-loaded entry.
 */
export function mergeWithPreview(entries: FileEntry[], previews: FileEntry[]): FileEntry[] {
  if (previews.length === 0) return sortFileEntries(entries);
  const previewPaths = new Set(previews.map((e) => e.path));
  const filtered = entries.filter((e) => !previewPaths.has(e.path));
  return sortFileEntries([...filtered, ...previews]);
}
