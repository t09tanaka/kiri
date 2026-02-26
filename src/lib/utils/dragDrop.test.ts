import { describe, it, expect } from 'vitest';
import {
  getParentDirectory,
  isDescendantOf,
  resolveDropTarget,
  isValidMoveTarget,
} from './dragDrop';

describe('getParentDirectory', () => {
  it('should return parent directory for a file path', () => {
    expect(getParentDirectory('/project/src/lib/utils/fileIcons.ts')).toBe(
      '/project/src/lib/utils'
    );
  });

  it('should return parent for a nested directory', () => {
    expect(getParentDirectory('/project/src/lib')).toBe('/project/src');
  });

  it('should return root for a top-level item', () => {
    expect(getParentDirectory('/project/file.txt')).toBe('/project');
  });

  it('should return slash for root-level path', () => {
    expect(getParentDirectory('/file.txt')).toBe('/');
  });

  it('should handle trailing slashes', () => {
    expect(getParentDirectory('/project/src/')).toBe('/project');
  });
});

describe('isDescendantOf', () => {
  it('returns true for direct child', () => {
    expect(isDescendantOf('/a/b', '/a')).toBe(true);
  });
  it('returns true for deeply nested descendant', () => {
    expect(isDescendantOf('/a/b/c/d', '/a')).toBe(true);
  });
  it('returns false for same path', () => {
    expect(isDescendantOf('/a', '/a')).toBe(false);
  });
  it('returns false for parent', () => {
    expect(isDescendantOf('/a', '/a/b')).toBe(false);
  });
  it('returns false for sibling', () => {
    expect(isDescendantOf('/a/c', '/a/b')).toBe(false);
  });
  it('returns false for prefix-similar paths', () => {
    expect(isDescendantOf('/project-backup/file', '/project')).toBe(false);
  });
});

describe('resolveDropTarget', () => {
  it('should return directory path when hovering over a directory', () => {
    expect(resolveDropTarget('/project/src', true, '/project')).toBe('/project/src');
  });

  it('should return parent directory when hovering over a file', () => {
    expect(resolveDropTarget('/project/src/app.ts', false, '/project')).toBe('/project/src');
  });

  it('should return rootPath when hovering over a root-level file', () => {
    expect(resolveDropTarget('/project/README.md', false, '/project')).toBe('/project');
  });

  it('should return null when path is null (no element found)', () => {
    expect(resolveDropTarget(null, false, '/project')).toBe(null);
  });

  it('should clamp to rootPath when parent is above rootPath', () => {
    expect(resolveDropTarget('/other/file.txt', false, '/project')).toBe('/project');
  });

  it('should clamp directory outside rootPath to rootPath', () => {
    expect(resolveDropTarget('/other/dir', true, '/project')).toBe('/project');
  });

  it('should not match adjacent root names with startsWith', () => {
    expect(resolveDropTarget('/project-backup/file.txt', false, '/project')).toBe('/project');
  });

  it('should not match adjacent root names for directories', () => {
    expect(resolveDropTarget('/project-backup/dir', true, '/project')).toBe('/project');
  });
});

describe('isValidMoveTarget', () => {
  it('returns false for null target', () => {
    expect(isValidMoveTarget(null, '/a/file.txt', false)).toBe(false);
  });
  it('returns false for same directory (source parent)', () => {
    expect(isValidMoveTarget('/a', '/a/file.txt', false)).toBe(false);
  });
  it('returns true for different directory', () => {
    expect(isValidMoveTarget('/b', '/a/file.txt', false)).toBe(true);
  });
  it('returns false for self-drop (dir onto itself)', () => {
    expect(isValidMoveTarget('/a/dir', '/a/dir', true)).toBe(false);
  });
  it('returns false for descendant drop', () => {
    expect(isValidMoveTarget('/a/dir/child', '/a/dir', true)).toBe(false);
  });
  it('allows moving dir to non-descendant', () => {
    expect(isValidMoveTarget('/b', '/a/dir', true)).toBe(true);
  });
});
