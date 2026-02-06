import { describe, expect, it } from 'vitest';
import {
  detectEmbeddedContext,
  escapeHtml,
  getLanguageFromPath,
  getLineLanguage,
  highlightLine,
  insertMarksIntoHighlightedHtml,
  supportsEmbeddedLanguages,
} from './syntaxHighlight';

describe('escapeHtml', () => {
  it('escapes HTML special characters', () => {
    expect(escapeHtml('<div class="test">')).toBe('&lt;div class=&quot;test&quot;&gt;');
  });

  it('escapes ampersands', () => {
    expect(escapeHtml('a & b')).toBe('a &amp; b');
  });

  it('escapes single quotes', () => {
    expect(escapeHtml("it's")).toBe('it&#039;s');
  });

  it('handles empty string', () => {
    expect(escapeHtml('')).toBe('');
  });

  it('returns unchanged string with no special characters', () => {
    expect(escapeHtml('hello world')).toBe('hello world');
  });
});

describe('getLanguageFromPath', () => {
  it('returns typescript for .ts files', () => {
    expect(getLanguageFromPath('src/main.ts')).toBe('typescript');
  });

  it('returns typescript for .tsx files', () => {
    expect(getLanguageFromPath('Component.tsx')).toBe('typescript');
  });

  it('returns javascript for .js files', () => {
    expect(getLanguageFromPath('index.js')).toBe('javascript');
  });

  it('returns rust for .rs files', () => {
    expect(getLanguageFromPath('main.rs')).toBe('rust');
  });

  it('returns json for .json files', () => {
    expect(getLanguageFromPath('package.json')).toBe('json');
  });

  it('returns svelte for .svelte files', () => {
    expect(getLanguageFromPath('App.svelte')).toBe('svelte');
  });

  it('returns null for unknown extensions', () => {
    expect(getLanguageFromPath('file.xyz')).toBeNull();
  });

  it('returns makefile for Makefile', () => {
    expect(getLanguageFromPath('Makefile')).toBe('makefile');
  });

  it('handles paths with multiple dots', () => {
    expect(getLanguageFromPath('src/lib/utils/file.test.ts')).toBe('typescript');
  });

  it('returns python for .py files', () => {
    expect(getLanguageFromPath('script.py')).toBe('python');
  });

  it('returns css for .css files', () => {
    expect(getLanguageFromPath('styles.css')).toBe('css');
  });

  it('returns yaml for .yml files', () => {
    expect(getLanguageFromPath('config.yml')).toBe('yaml');
  });

  it('returns bash for .sh files', () => {
    expect(getLanguageFromPath('build.sh')).toBe('bash');
  });

  it('returns go for .go files', () => {
    expect(getLanguageFromPath('main.go')).toBe('go');
  });
});

describe('highlightLine', () => {
  it('returns escaped HTML when language is null', () => {
    expect(highlightLine('<div>', null)).toBe('&lt;div&gt;');
  });

  it('returns escaped HTML for empty content', () => {
    expect(highlightLine('', 'typescript')).toBe('');
  });

  it('highlights TypeScript code', () => {
    const result = highlightLine('const x = 42;', 'typescript');
    expect(result).toContain('hljs-');
    expect(result).toContain('const');
    expect(result).toContain('42');
  });

  it('highlights Rust code', () => {
    const result = highlightLine('let mut x = 5;', 'rust');
    expect(result).toContain('hljs-');
    expect(result).toContain('let');
  });

  it('handles invalid language gracefully', () => {
    const result = highlightLine('some code', 'nonexistent_language');
    expect(result).toBe('some code');
  });

  it('preserves content in highlighted output', () => {
    const result = highlightLine('function hello() {}', 'javascript');
    // The text content should be preserved even if wrapped in spans
    const textContent = result.replace(/<[^>]*>/g, '');
    expect(textContent).toBe('function hello() {}');
  });
});

describe('supportsEmbeddedLanguages', () => {
  it('returns true for .svelte files', () => {
    expect(supportsEmbeddedLanguages('App.svelte')).toBe(true);
  });

  it('returns true for .vue files', () => {
    expect(supportsEmbeddedLanguages('Component.vue')).toBe(true);
  });

  it('returns true for .html files', () => {
    expect(supportsEmbeddedLanguages('index.html')).toBe(true);
  });

  it('returns true for .htm files', () => {
    expect(supportsEmbeddedLanguages('page.htm')).toBe(true);
  });

  it('returns false for .ts files', () => {
    expect(supportsEmbeddedLanguages('main.ts')).toBe(false);
  });

  it('returns false for .rs files', () => {
    expect(supportsEmbeddedLanguages('lib.rs')).toBe(false);
  });
});

describe('getLineLanguage', () => {
  it('detects script opening tag', () => {
    const result = getLineLanguage('<script lang="ts">', 'svelte', 'template');
    expect(result.language).toBe('typescript');
    expect(result.newContext).toBe('script');
  });

  it('detects script closing tag', () => {
    const result = getLineLanguage('</script>', 'svelte', 'script');
    expect(result.language).toBe('typescript');
    expect(result.newContext).toBe('template');
  });

  it('detects style opening tag', () => {
    const result = getLineLanguage('<style>', 'svelte', 'template');
    expect(result.language).toBe('css');
    expect(result.newContext).toBe('style');
  });

  it('detects style closing tag', () => {
    const result = getLineLanguage('</style>', 'svelte', 'style');
    expect(result.language).toBe('css');
    expect(result.newContext).toBe('template');
  });

  it('returns typescript for script context', () => {
    const result = getLineLanguage('const x = 5;', 'svelte', 'script');
    expect(result.language).toBe('typescript');
    expect(result.newContext).toBe('script');
  });

  it('returns css for style context', () => {
    const result = getLineLanguage('.class { color: red; }', 'svelte', 'style');
    expect(result.language).toBe('css');
    expect(result.newContext).toBe('style');
  });

  it('returns base language for template context', () => {
    const result = getLineLanguage('<div>Hello</div>', 'svelte', 'template');
    expect(result.language).toBe('svelte');
    expect(result.newContext).toBe('template');
  });
});

describe('detectEmbeddedContext', () => {
  it('detects script context from TypeScript keywords', () => {
    expect(detectEmbeddedContext(['const x = 5;', 'let y = 10;'])).toBe('script');
    expect(detectEmbeddedContext(['import { foo } from "bar";'])).toBe('script');
    expect(detectEmbeddedContext(['export function hello() {}'])).toBe('script');
    expect(detectEmbeddedContext(['function handleClick() {'])).toBe('script');
    expect(detectEmbeddedContext(['class MyComponent {'])).toBe('script');
    expect(detectEmbeddedContext(['interface Props {'])).toBe('script');
    expect(detectEmbeddedContext(['type Foo = string;'])).toBe('script');
    expect(detectEmbeddedContext(['async function load() {'])).toBe('script');
    expect(detectEmbeddedContext(['return result;'])).toBe('script');
    expect(detectEmbeddedContext(['throw new Error("fail");'])).toBe('script');
    expect(detectEmbeddedContext(['await fetch(url);'])).toBe('script');
  });

  it('detects script context from Svelte runes and reactive declarations', () => {
    expect(detectEmbeddedContext(['$: count = items.length;'])).toBe('script');
    expect(detectEmbeddedContext(['$state(0)'])).toBe('script');
    expect(detectEmbeddedContext(['$derived(getTotal())'])).toBe('script');
    expect(detectEmbeddedContext(['$effect(() => {'])).toBe('script');
    expect(detectEmbeddedContext(['$props()'])).toBe('script');
  });

  it('detects template context from HTML elements', () => {
    expect(detectEmbeddedContext(['<div class="container">'])).toBe('template');
    expect(detectEmbeddedContext(['<span>text</span>'])).toBe('template');
    expect(detectEmbeddedContext(['</button>'])).toBe('template');
    expect(detectEmbeddedContext(['<svg width="24">'])).toBe('template');
  });

  it('detects template context from Svelte template blocks', () => {
    expect(detectEmbeddedContext(['{#if condition}'])).toBe('template');
    expect(detectEmbeddedContext(['{:else}'])).toBe('template');
    expect(detectEmbeddedContext(['{/each}'])).toBe('template');
  });

  it('detects script context from explicit script tag', () => {
    expect(detectEmbeddedContext(['<script lang="ts">'])).toBe('script');
  });

  it('detects template context from closing script tag', () => {
    expect(detectEmbeddedContext(['</script>'])).toBe('template');
  });

  it('detects style context from explicit style tag', () => {
    expect(detectEmbeddedContext(['<style>'])).toBe('style');
  });

  it('detects template context from closing style tag', () => {
    expect(detectEmbeddedContext(['</style>'])).toBe('template');
  });

  it('detects style context from CSS patterns', () => {
    expect(detectEmbeddedContext(['color: red;'])).toBe('style');
    expect(detectEmbeddedContext(['.container {'])).toBe('style');
    expect(detectEmbeddedContext(['#main {'])).toBe('style');
    expect(detectEmbeddedContext(['div {'])).toBe('style');
  });

  it('skips empty lines when detecting context', () => {
    expect(detectEmbeddedContext(['', '  ', 'const x = 5;'])).toBe('script');
    expect(detectEmbeddedContext(['', '<div>'])).toBe('template');
  });

  it('returns template as default for empty input', () => {
    expect(detectEmbeddedContext([])).toBe('template');
    expect(detectEmbeddedContext(['', '  '])).toBe('template');
  });

  it('does not confuse arrow functions with CSS', () => {
    // "=>" contains ":" patterns but should not be detected as CSS
    expect(detectEmbeddedContext(['const fn = (x) => x + 1;'])).toBe('script');
  });

  it('uses first non-empty line for detection', () => {
    // First meaningful line is HTML, even if later lines look like TS
    expect(detectEmbeddedContext(['<div>', 'const x = 5;'])).toBe('template');
    // First meaningful line is TS, even if later lines look like HTML
    expect(detectEmbeddedContext(['const x = 5;', '<div>'])).toBe('script');
  });
});

describe('insertMarksIntoHighlightedHtml', () => {
  it('returns html unchanged when no marks', () => {
    const html = '<span class="hljs-keyword">const</span>';
    expect(insertMarksIntoHighlightedHtml(html, [])).toBe(html);
  });

  it('inserts mark in plain text', () => {
    const html = 'hello world';
    const result = insertMarksIntoHighlightedHtml(html, [{ start: 6, end: 11 }]);
    expect(result).toBe('hello <mark>world</mark>');
  });

  it('inserts mark inside a span', () => {
    // "const" is 5 chars (c=0,o=1,n=2,s=3,t=4), highlighted as keyword
    const html = '<span class="hljs-keyword">const</span> x';
    const result = insertMarksIntoHighlightedHtml(html, [{ start: 2, end: 4 }]);
    // mark covers positions 2-3 (n,s), t at position 4 remains outside mark
    expect(result).toBe('<span class="hljs-keyword">co<mark>ns</mark>t</span> x');
  });

  it('inserts mark spanning across span boundary', () => {
    // "const x" - "const" in span, " x" outside
    const html = '<span class="hljs-keyword">const</span> x';
    // Mark covers "st x" (positions 3-7)
    const result = insertMarksIntoHighlightedHtml(html, [{ start: 3, end: 7 }]);
    // Should close mark before closing span, then reopen after
    expect(result).toBe('<span class="hljs-keyword">con<mark>st</mark></span><mark> x</mark>');
  });

  it('handles multiple marks', () => {
    const html = 'aaa bbb ccc';
    const result = insertMarksIntoHighlightedHtml(html, [
      { start: 0, end: 3 },
      { start: 8, end: 11 },
    ]);
    expect(result).toBe('<mark>aaa</mark> bbb <mark>ccc</mark>');
  });

  it('handles HTML entities in highlighted text', () => {
    // highlight.js escapes < to &lt;
    const html = '&lt;div&gt;';
    // Original text is "<div>", mark the "div" part (positions 1-4)
    const result = insertMarksIntoHighlightedHtml(html, [{ start: 1, end: 4 }]);
    expect(result).toBe('&lt;<mark>div</mark>&gt;');
  });

  it('handles mark at the start of text', () => {
    const html = '<span class="hljs-keyword">const</span>';
    const result = insertMarksIntoHighlightedHtml(html, [{ start: 0, end: 5 }]);
    expect(result).toBe('<span class="hljs-keyword"><mark>const</mark></span>');
  });

  it('handles mark covering entire text with span', () => {
    const html = '<span class="hljs-keyword">const</span> x = <span class="hljs-number">42</span>';
    // Mark covers all "const x = 42" (0-12)
    const result = insertMarksIntoHighlightedHtml(html, [{ start: 0, end: 12 }]);
    expect(result).toContain('<mark>');
    expect(result).toContain('</mark>');
    // Ensure all text is within marks
    const textContent = result.replace(/<[^>]*>/g, '');
    expect(textContent).toBe('const x = 42');
  });

  it('handles adjacent marks', () => {
    const html = 'abcdef';
    const result = insertMarksIntoHighlightedHtml(html, [
      { start: 0, end: 3 },
      { start: 3, end: 6 },
    ]);
    expect(result).toBe('<mark>abc</mark><mark>def</mark>');
  });

  it('handles empty span tags', () => {
    const html = '<span class="hljs-keyword"></span>text';
    const result = insertMarksIntoHighlightedHtml(html, [{ start: 0, end: 4 }]);
    // Opening span is emitted first, then mark opens after closing span
    expect(result).toBe('<span class="hljs-keyword"></span><mark>text</mark>');
  });
});
