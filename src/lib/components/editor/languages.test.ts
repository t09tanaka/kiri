import { describe, it, expect } from 'vitest';
import { getLanguageExtension, getLanguageName } from './languages';

describe('languages', () => {
  describe('getLanguageExtension', () => {
    it('should return TypeScript extension for .ts files', async () => {
      const ext = await getLanguageExtension('file.ts');
      expect(ext).not.toBeNull();
    });

    it('should return TypeScript JSX extension for .tsx files', async () => {
      const ext = await getLanguageExtension('file.tsx');
      expect(ext).not.toBeNull();
    });

    it('should return JavaScript extension for .js files', async () => {
      const ext = await getLanguageExtension('file.js');
      expect(ext).not.toBeNull();
    });

    it('should return JavaScript JSX extension for .jsx files', async () => {
      const ext = await getLanguageExtension('file.jsx');
      expect(ext).not.toBeNull();
    });

    it('should return Rust extension for .rs files', async () => {
      const ext = await getLanguageExtension('file.rs');
      expect(ext).not.toBeNull();
    });

    it('should return JSON extension for .json files', async () => {
      const ext = await getLanguageExtension('file.json');
      expect(ext).not.toBeNull();
    });

    it('should return Markdown extension for .md files', async () => {
      const ext = await getLanguageExtension('file.md');
      expect(ext).not.toBeNull();
    });

    it('should return CSS extension for .css files', async () => {
      const ext = await getLanguageExtension('file.css');
      expect(ext).not.toBeNull();
    });

    it('should return CSS extension for .scss files', async () => {
      const ext = await getLanguageExtension('file.scss');
      expect(ext).not.toBeNull();
    });

    it('should return HTML extension for .html files', async () => {
      const ext = await getLanguageExtension('file.html');
      expect(ext).not.toBeNull();
    });

    it('should return HTML extension for .svelte files', async () => {
      const ext = await getLanguageExtension('file.svelte');
      expect(ext).not.toBeNull();
    });

    it('should return YAML extension for .yaml files', async () => {
      const ext = await getLanguageExtension('file.yaml');
      expect(ext).not.toBeNull();
    });

    it('should return YAML extension for .yml files', async () => {
      const ext = await getLanguageExtension('file.yml');
      expect(ext).not.toBeNull();
    });

    it('should return null for unknown extensions', async () => {
      const ext = await getLanguageExtension('file.unknown');
      expect(ext).toBeNull();
    });

    it('should return null for files without extension', async () => {
      const ext = await getLanguageExtension('noextension');
      expect(ext).toBeNull();
    });

    it('should handle uppercase extensions', async () => {
      const ext = await getLanguageExtension('file.TS');
      expect(ext).not.toBeNull();
    });

    it('should cache loaded extensions', async () => {
      // Load same extension twice
      const ext1 = await getLanguageExtension('file1.ts');
      const ext2 = await getLanguageExtension('file2.ts');
      // Both should return the same cached extension
      expect(ext1).toBe(ext2);
    });
  });

  describe('getLanguageName', () => {
    it('should return TypeScript for .ts files', () => {
      expect(getLanguageName('file.ts')).toBe('TypeScript');
    });

    it('should return TypeScript React for .tsx files', () => {
      expect(getLanguageName('file.tsx')).toBe('TypeScript React');
    });

    it('should return JavaScript for .js files', () => {
      expect(getLanguageName('file.js')).toBe('JavaScript');
    });

    it('should return JavaScript React for .jsx files', () => {
      expect(getLanguageName('file.jsx')).toBe('JavaScript React');
    });

    it('should return Rust for .rs files', () => {
      expect(getLanguageName('file.rs')).toBe('Rust');
    });

    it('should return JSON for .json files', () => {
      expect(getLanguageName('file.json')).toBe('JSON');
    });

    it('should return Markdown for .md files', () => {
      expect(getLanguageName('file.md')).toBe('Markdown');
    });

    it('should return CSS for .css files', () => {
      expect(getLanguageName('file.css')).toBe('CSS');
    });

    it('should return SCSS for .scss files', () => {
      expect(getLanguageName('file.scss')).toBe('SCSS');
    });

    it('should return HTML for .html files', () => {
      expect(getLanguageName('file.html')).toBe('HTML');
    });

    it('should return Svelte for .svelte files', () => {
      expect(getLanguageName('file.svelte')).toBe('Svelte');
    });

    it('should return TOML for .toml files', () => {
      expect(getLanguageName('file.toml')).toBe('TOML');
    });

    it('should return YAML for .yaml files', () => {
      expect(getLanguageName('file.yaml')).toBe('YAML');
    });

    it('should return YAML for .yml files', () => {
      expect(getLanguageName('file.yml')).toBe('YAML');
    });

    it('should return Plain Text for unknown extensions', () => {
      expect(getLanguageName('file.xyz')).toBe('Plain Text');
    });

    it('should return Plain Text for files without extension', () => {
      expect(getLanguageName('noextension')).toBe('Plain Text');
    });

    it('should handle uppercase extensions', () => {
      expect(getLanguageName('file.TS')).toBe('TypeScript');
    });

    it('should handle path with multiple dots', () => {
      expect(getLanguageName('path/to/file.test.ts')).toBe('TypeScript');
    });
  });
});
