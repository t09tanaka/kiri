import hljs from 'highlight.js/lib/core';

// Import only the languages we need to keep bundle size small
import javascript from 'highlight.js/lib/languages/javascript';
import typescript from 'highlight.js/lib/languages/typescript';
import xml from 'highlight.js/lib/languages/xml';
import css from 'highlight.js/lib/languages/css';
import json from 'highlight.js/lib/languages/json';
import rust from 'highlight.js/lib/languages/rust';
import yaml from 'highlight.js/lib/languages/yaml';
import markdown from 'highlight.js/lib/languages/markdown';
import bash from 'highlight.js/lib/languages/bash';
import python from 'highlight.js/lib/languages/python';
import go from 'highlight.js/lib/languages/go';
import php from 'highlight.js/lib/languages/php';
import ruby from 'highlight.js/lib/languages/ruby';
import java from 'highlight.js/lib/languages/java';
import kotlin from 'highlight.js/lib/languages/kotlin';
import swift from 'highlight.js/lib/languages/swift';
import c from 'highlight.js/lib/languages/c';
import cpp from 'highlight.js/lib/languages/cpp';
import csharp from 'highlight.js/lib/languages/csharp';
import sql from 'highlight.js/lib/languages/sql';
import graphql from 'highlight.js/lib/languages/graphql';
import dockerfile from 'highlight.js/lib/languages/dockerfile';
import ini from 'highlight.js/lib/languages/ini';
import makefile from 'highlight.js/lib/languages/makefile';
import diff from 'highlight.js/lib/languages/diff';
import plaintext from 'highlight.js/lib/languages/plaintext';
import perl from 'highlight.js/lib/languages/perl';
import lua from 'highlight.js/lib/languages/lua';
import r from 'highlight.js/lib/languages/r';
import scala from 'highlight.js/lib/languages/scala';
import haskell from 'highlight.js/lib/languages/haskell';
import elixir from 'highlight.js/lib/languages/elixir';
import erlang from 'highlight.js/lib/languages/erlang';
import clojure from 'highlight.js/lib/languages/clojure';
import fsharp from 'highlight.js/lib/languages/fsharp';
import ocaml from 'highlight.js/lib/languages/ocaml';
import dart from 'highlight.js/lib/languages/dart';
import objectivec from 'highlight.js/lib/languages/objectivec';
import latex from 'highlight.js/lib/languages/latex';
import wasm from 'highlight.js/lib/languages/wasm';
import protobuf from 'highlight.js/lib/languages/protobuf';
import nginx from 'highlight.js/lib/languages/nginx';
import apache from 'highlight.js/lib/languages/apache';

// Register languages
hljs.registerLanguage('javascript', javascript);
hljs.registerLanguage('typescript', typescript);
hljs.registerLanguage('xml', xml);
hljs.registerLanguage('html', xml);
hljs.registerLanguage('svelte', xml);
hljs.registerLanguage('vue', xml);
hljs.registerLanguage('css', css);
hljs.registerLanguage('scss', css);
hljs.registerLanguage('less', css);
hljs.registerLanguage('json', json);
hljs.registerLanguage('rust', rust);
hljs.registerLanguage('yaml', yaml);
hljs.registerLanguage('markdown', markdown);
hljs.registerLanguage('bash', bash);
hljs.registerLanguage('shell', bash);
hljs.registerLanguage('python', python);
hljs.registerLanguage('go', go);
hljs.registerLanguage('php', php);
hljs.registerLanguage('ruby', ruby);
hljs.registerLanguage('java', java);
hljs.registerLanguage('kotlin', kotlin);
hljs.registerLanguage('swift', swift);
hljs.registerLanguage('c', c);
hljs.registerLanguage('cpp', cpp);
hljs.registerLanguage('csharp', csharp);
hljs.registerLanguage('sql', sql);
hljs.registerLanguage('graphql', graphql);
hljs.registerLanguage('dockerfile', dockerfile);
hljs.registerLanguage('ini', ini);
hljs.registerLanguage('toml', ini);
hljs.registerLanguage('makefile', makefile);
hljs.registerLanguage('diff', diff);
hljs.registerLanguage('plaintext', plaintext);
hljs.registerLanguage('perl', perl);
hljs.registerLanguage('lua', lua);
hljs.registerLanguage('r', r);
hljs.registerLanguage('scala', scala);
hljs.registerLanguage('haskell', haskell);
hljs.registerLanguage('elixir', elixir);
hljs.registerLanguage('erlang', erlang);
hljs.registerLanguage('clojure', clojure);
hljs.registerLanguage('fsharp', fsharp);
hljs.registerLanguage('ocaml', ocaml);
hljs.registerLanguage('dart', dart);
hljs.registerLanguage('objectivec', objectivec);
hljs.registerLanguage('latex', latex);
hljs.registerLanguage('wasm', wasm);
hljs.registerLanguage('protobuf', protobuf);
hljs.registerLanguage('nginx', nginx);
hljs.registerLanguage('apache', apache);

// Map file extensions to highlight.js language names
const EXTENSION_LANG_MAP: Record<string, string> = {
  // JavaScript/TypeScript
  ts: 'typescript',
  tsx: 'typescript',
  mts: 'typescript',
  cts: 'typescript',
  js: 'javascript',
  jsx: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',
  // Web frameworks
  svelte: 'svelte',
  vue: 'vue',
  // Markup
  html: 'html',
  htm: 'html',
  xml: 'xml',
  svg: 'xml',
  xhtml: 'xml',
  // Styles
  css: 'css',
  scss: 'scss',
  sass: 'scss',
  less: 'less',
  // Data formats
  json: 'json',
  jsonc: 'json',
  json5: 'json',
  yaml: 'yaml',
  yml: 'yaml',
  toml: 'toml',
  ini: 'ini',
  // Systems programming
  rs: 'rust',
  c: 'c',
  h: 'c',
  cpp: 'cpp',
  cc: 'cpp',
  cxx: 'cpp',
  hpp: 'cpp',
  hxx: 'cpp',
  // JVM languages
  java: 'java',
  kt: 'kotlin',
  kts: 'kotlin',
  scala: 'scala',
  clj: 'clojure',
  cljs: 'clojure',
  cljc: 'clojure',
  // .NET
  cs: 'csharp',
  fs: 'fsharp',
  fsx: 'fsharp',
  // Apple platforms
  swift: 'swift',
  m: 'objectivec',
  mm: 'objectivec',
  // Scripting languages
  py: 'python',
  pyw: 'python',
  rb: 'ruby',
  rake: 'ruby',
  php: 'php',
  pl: 'perl',
  pm: 'perl',
  lua: 'lua',
  r: 'r',
  R: 'r',
  // Functional languages
  hs: 'haskell',
  lhs: 'haskell',
  ml: 'ocaml',
  mli: 'ocaml',
  ex: 'elixir',
  exs: 'elixir',
  erl: 'erlang',
  hrl: 'erlang',
  // Mobile
  dart: 'dart',
  // Go
  go: 'go',
  // Shell
  sh: 'bash',
  bash: 'bash',
  zsh: 'bash',
  fish: 'bash',
  // Database
  sql: 'sql',
  mysql: 'sql',
  pgsql: 'sql',
  // API/Schema
  graphql: 'graphql',
  gql: 'graphql',
  proto: 'protobuf',
  // Config/Build
  dockerfile: 'dockerfile',
  makefile: 'makefile',
  mk: 'makefile',
  // Documentation
  md: 'markdown',
  mdx: 'markdown',
  tex: 'latex',
  // Other
  diff: 'diff',
  patch: 'diff',
  wasm: 'wasm',
  wat: 'wasm',
  txt: 'plaintext',
  // Server config
  conf: 'nginx',
  nginx: 'nginx',
  htaccess: 'apache',
};

export function getLanguageFromPath(path: string): string | null {
  const ext = path.split('.').pop()?.toLowerCase();
  return ext ? EXTENSION_LANG_MAP[ext] || null : null;
}

export function highlightLine(content: string, language: string | null): string {
  if (!content || !language) {
    return escapeHtml(content);
  }
  try {
    const result = hljs.highlight(content, { language, ignoreIllegals: true });
    return result.value;
  } catch {
    return escapeHtml(content);
  }
}

export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

export type EmbeddedContext = 'script' | 'style' | 'template';

// Tag patterns for detecting context changes (split to avoid Svelte parser issues)
const SCRIPT_OPEN = '<' + 'script';
const SCRIPT_CLOSE = '</' + 'script>';
const STYLE_OPEN = '<' + 'style';
const STYLE_CLOSE = '</' + 'style>';

export function supportsEmbeddedLanguages(path: string): boolean {
  const ext = path.split('.').pop()?.toLowerCase();
  return ext === 'svelte' || ext === 'vue' || ext === 'html' || ext === 'htm';
}

export function getLineLanguage(
  content: string,
  baseLanguage: string | null,
  currentContext: EmbeddedContext
): { language: string | null; newContext: EmbeddedContext } {
  if (content.includes(SCRIPT_OPEN) && content.includes('>')) {
    return { language: 'typescript', newContext: 'script' };
  }
  if (content.includes(SCRIPT_CLOSE)) {
    return { language: 'typescript', newContext: 'template' };
  }
  if (content.includes(STYLE_OPEN) && content.includes('>')) {
    return { language: 'css', newContext: 'style' };
  }
  if (content.includes(STYLE_CLOSE)) {
    return { language: 'css', newContext: 'template' };
  }

  switch (currentContext) {
    case 'script':
      return { language: 'typescript', newContext: 'script' };
    case 'style':
      return { language: 'css', newContext: 'style' };
    default:
      return { language: baseLanguage, newContext: 'template' };
  }
}

/**
 * Detect the embedded language context by analyzing content lines.
 * Used for diff hunks where the section boundary (<script>, <style>) may not be visible.
 * Examines up to the first non-empty lines to determine whether the code is
 * TypeScript (script), HTML (template), or CSS (style).
 */
export function detectEmbeddedContext(contentLines: string[]): EmbeddedContext {
  for (const line of contentLines) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    // Explicit section boundaries
    if (trimmed.includes(SCRIPT_OPEN)) return 'script';
    if (trimmed.includes(SCRIPT_CLOSE)) return 'template';
    if (trimmed.includes(STYLE_OPEN)) return 'style';
    if (trimmed.includes(STYLE_CLOSE)) return 'template';

    // TypeScript/JavaScript patterns
    if (
      /^(import|export|const|let|var|function|class|interface|type|async|return|throw|await)\b/.test(
        trimmed
      )
    ) {
      return 'script';
    }
    // Svelte reactive declarations ($:) and rune calls ($state, $derived, $effect, $props)
    if (/^\$[:(\w]/.test(trimmed)) {
      return 'script';
    }

    // HTML/Template patterns
    if (/^<[a-zA-Z]/.test(trimmed) || /^<\/[a-zA-Z]/.test(trimmed)) {
      return 'template';
    }
    // Svelte template blocks: {#if}, {:else}, {/each}, etc.
    if (/^\{[#:/]/.test(trimmed)) {
      return 'template';
    }

    // CSS patterns: property declarations like "color: red;" or selectors like ".class {"
    if (/^[a-z][\w-]*\s*:\s*[^=]/.test(trimmed) && !trimmed.includes('=>')) {
      return 'style';
    }
    if (/^[.#&@][\w-]/.test(trimmed) || /^\w[\w-]*\s*\{/.test(trimmed)) {
      return 'style';
    }
  }

  return 'template';
}

export interface SearchMark {
  start: number;
  end: number;
}

/**
 * Insert <mark> tags into syntax-highlighted HTML at the correct text positions.
 *
 * Walk through the HTML tracking the position in the original (unescaped) text.
 * HTML tags (<span>, </span>) and entities (&amp; etc.) are handled so that
 * marks are inserted at the right character boundaries.
 */
export function insertMarksIntoHighlightedHtml(
  highlightedHtml: string,
  marks: SearchMark[]
): string {
  if (marks.length === 0) return highlightedHtml;

  const sorted = [...marks].sort((a, b) => a.start - b.start);
  let result = '';
  let textPos = 0;
  let markIdx = 0;
  let inMark = false;
  let i = 0;

  // Helper: check if current position is an HTML tag and return it
  function peekTag(): { tag: string; end: number; isClosing: boolean } | null {
    if (i >= highlightedHtml.length || highlightedHtml[i] !== '<') return null;
    const closeIdx = highlightedHtml.indexOf('>', i);
    if (closeIdx === -1) return null;
    const tag = highlightedHtml.slice(i, closeIdx + 1);
    return { tag, end: closeIdx + 1, isClosing: tag.startsWith('</') };
  }

  while (i < highlightedHtml.length) {
    // Step 1: Close mark if we've reached end position
    if (inMark && markIdx < sorted.length && textPos >= sorted[markIdx].end) {
      result += '</mark>';
      inMark = false;
      markIdx++;
    }

    // Step 2: Process opening tags before opening marks (ensures marks nest inside spans)
    const tagInfo = peekTag();
    if (tagInfo && !tagInfo.isClosing) {
      result += tagInfo.tag;
      i = tagInfo.end;
      continue;
    }

    // Step 3: Process closing tags with mark awareness
    if (tagInfo && tagInfo.isClosing) {
      if (inMark) {
        // Close mark before closing tag, reopen after for proper nesting
        result += '</mark>';
        result += tagInfo.tag;
        result += '<mark>';
      } else {
        result += tagInfo.tag;
      }
      i = tagInfo.end;
      continue;
    }

    // Step 4: Open mark if we've reached start position (after tags are processed)
    if (
      !inMark &&
      markIdx < sorted.length &&
      textPos >= sorted[markIdx].start &&
      textPos < sorted[markIdx].end
    ) {
      result += '<mark>';
      inMark = true;
    }

    // Step 5: Process text character or HTML entity
    if (highlightedHtml[i] === '&') {
      const semiIdx = highlightedHtml.indexOf(';', i);
      if (semiIdx !== -1 && semiIdx - i < 10) {
        result += highlightedHtml.slice(i, semiIdx + 1);
        i = semiIdx + 1;
        textPos++;
      } else {
        result += highlightedHtml[i];
        i++;
        textPos++;
      }
    } else {
      result += highlightedHtml[i];
      i++;
      textPos++;
    }
  }

  if (inMark) {
    result += '</mark>';
  }

  return result;
}
