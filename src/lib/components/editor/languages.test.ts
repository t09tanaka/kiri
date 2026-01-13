import { describe, it, expect } from 'vitest';
import { getLanguageExtension, getLanguageName } from './languages';

describe('languages', () => {
  describe('getLanguageExtension', () => {
    it('should return TypeScript extension for .ts files', () => {
      const ext = getLanguageExtension('file.ts');
      expect(ext).not.toBeNull();
    });

    it('should return TypeScript JSX extension for .tsx files', () => {
      const ext = getLanguageExtension('file.tsx');
      expect(ext).not.toBeNull();
    });

    it('should return JavaScript extension for .js files', () => {
      const ext = getLanguageExtension('file.js');
      expect(ext).not.toBeNull();
    });

    it('should return JavaScript JSX extension for .jsx files', () => {
      const ext = getLanguageExtension('file.jsx');
      expect(ext).not.toBeNull();
    });

    it('should return Rust extension for .rs files', () => {
      const ext = getLanguageExtension('file.rs');
      expect(ext).not.toBeNull();
    });

    it('should return JSON extension for .json files', () => {
      const ext = getLanguageExtension('file.json');
      expect(ext).not.toBeNull();
    });

    it('should return Markdown extension for .md files', () => {
      const ext = getLanguageExtension('file.md');
      expect(ext).not.toBeNull();
    });

    it('should return CSS extension for .css files', () => {
      const ext = getLanguageExtension('file.css');
      expect(ext).not.toBeNull();
    });

    it('should return CSS extension for .scss files', () => {
      const ext = getLanguageExtension('file.scss');
      expect(ext).not.toBeNull();
    });

    it('should return HTML extension for .html files', () => {
      const ext = getLanguageExtension('file.html');
      expect(ext).not.toBeNull();
    });

    it('should return HTML extension for .svelte files', () => {
      const ext = getLanguageExtension('file.svelte');
      expect(ext).not.toBeNull();
    });

    it('should return null for unknown extensions', () => {
      const ext = getLanguageExtension('file.unknown');
      expect(ext).toBeNull();
    });

    it('should return null for files without extension', () => {
      const ext = getLanguageExtension('noextension');
      expect(ext).toBeNull();
    });

    it('should handle uppercase extensions', () => {
      const ext = getLanguageExtension('file.TS');
      expect(ext).not.toBeNull();
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
