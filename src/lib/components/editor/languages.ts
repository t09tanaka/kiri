import type { Extension } from '@codemirror/state';

// Cache for loaded language extensions to avoid re-importing
const languageCache = new Map<string, Extension>();

/**
 * Per-extension loader table.
 *
 * Each entry is an arrow that performs the dynamic `import()` lazily,
 * so the eight `@codemirror/lang-*` chunks stay off the startup graph
 * until a file of that type is actually opened. The keys are the file
 * extension (lowercase, no leading dot).
 *
 * Re-using a single loader for `ts`/`tsx` and `js`/`jsx` means Rollup
 * emits one `lang-javascript` chunk that the four extensions share,
 * instead of duplicating the parser bytes per call site.
 */
const langLoaders: Record<string, () => Promise<Extension>> = {
  ts: async () => (await import('@codemirror/lang-javascript')).javascript({ typescript: true }),
  tsx: async () =>
    (await import('@codemirror/lang-javascript')).javascript({ typescript: true, jsx: true }),
  js: async () => (await import('@codemirror/lang-javascript')).javascript(),
  jsx: async () => (await import('@codemirror/lang-javascript')).javascript({ jsx: true }),
  rs: async () => (await import('@codemirror/lang-rust')).rust(),
  json: async () => (await import('@codemirror/lang-json')).json(),
  md: async () => (await import('@codemirror/lang-markdown')).markdown(),
  css: async () => (await import('@codemirror/lang-css')).css(),
  scss: async () => (await import('@codemirror/lang-css')).css(),
  html: async () => (await import('@codemirror/lang-html')).html(),
  svelte: async () => (await import('@codemirror/lang-html')).html(),
  yaml: async () => (await import('@codemirror/lang-yaml')).yaml(),
  yml: async () => (await import('@codemirror/lang-yaml')).yaml(),
};

/**
 * Get the file extension from a filename
 */
function getFileExtension(filename: string): string | undefined {
  return filename.split('.').pop()?.toLowerCase();
}

/**
 * Get language extension for CodeMirror editor (async, lazy-loaded).
 * Languages are loaded on-demand via {@link langLoaders} and cached.
 */
export async function getLanguageExtension(filename: string): Promise<Extension | null> {
  const ext = getFileExtension(filename);
  if (!ext) return null;

  const cached = languageCache.get(ext);
  if (cached !== undefined) {
    return cached;
  }

  const loader = langLoaders[ext];
  if (!loader) {
    return null;
  }

  const langExt = await loader();
  languageCache.set(ext, langExt);
  return langExt;
}

export function getLanguageName(filename: string): string {
  const ext = getFileExtension(filename);

  switch (ext) {
    case 'ts':
      return 'TypeScript';
    case 'tsx':
      return 'TypeScript React';
    case 'js':
      return 'JavaScript';
    case 'jsx':
      return 'JavaScript React';
    case 'rs':
      return 'Rust';
    case 'json':
      return 'JSON';
    case 'md':
      return 'Markdown';
    case 'css':
      return 'CSS';
    case 'scss':
      return 'SCSS';
    case 'html':
      return 'HTML';
    case 'svelte':
      return 'Svelte';
    case 'toml':
      return 'TOML';
    case 'yaml':
    case 'yml':
      return 'YAML';
    default:
      return 'Plain Text';
  }
}
