import { describe, it, expect } from 'vitest';
import { getParentDirectory, resolveDropTarget } from './dragDrop';

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
});
