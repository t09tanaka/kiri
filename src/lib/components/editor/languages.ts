import type { Extension } from '@codemirror/state';

// Cache for loaded language extensions to avoid re-importing

const languageCache = new Map<string, Extension>();

/**
 * Get the file extension from a filename
 */
function getFileExtension(filename: string): string | undefined {
  return filename.split('.').pop()?.toLowerCase();
}

/**
 * Get language extension for CodeMirror editor (async, lazy-loaded)
 * Languages are loaded on-demand and cached for reuse
 */
export async function getLanguageExtension(filename: string): Promise<Extension | null> {
  const ext = getFileExtension(filename);
  if (!ext) return null;

  // Check cache first
  const cacheKey = ext;
  if (languageCache.has(cacheKey)) {
    return languageCache.get(cacheKey) || null;
  }

  let langExt: Extension | null = null;

  switch (ext) {
    case 'ts':
    case 'tsx': {
      const { javascript } = await import('@codemirror/lang-javascript');
      langExt = javascript({ typescript: true, jsx: ext === 'tsx' });
      break;
    }
    case 'js':
    case 'jsx': {
      const { javascript } = await import('@codemirror/lang-javascript');
      langExt = javascript({ jsx: ext === 'jsx' });
      break;
    }
    case 'rs': {
      const { rust } = await import('@codemirror/lang-rust');
      langExt = rust();
      break;
    }
    case 'json': {
      const { json } = await import('@codemirror/lang-json');
      langExt = json();
      break;
    }
    case 'md': {
      const { markdown } = await import('@codemirror/lang-markdown');
      langExt = markdown();
      break;
    }
    case 'css':
    case 'scss': {
      const { css } = await import('@codemirror/lang-css');
      langExt = css();
      break;
    }
    case 'html':
    case 'svelte': {
      const { html } = await import('@codemirror/lang-html');
      langExt = html();
      break;
    }
    case 'yaml':
    case 'yml': {
      const { yaml } = await import('@codemirror/lang-yaml');
      langExt = yaml();
      break;
    }
    default:
      return null;
  }

  // Cache the loaded extension
  if (langExt) {
    languageCache.set(cacheKey, langExt);
  }

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
