import { describe, it, expect } from 'vitest';
import { getParentDirectory } from './dragDrop';

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
