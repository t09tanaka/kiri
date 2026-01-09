import { javascript } from '@codemirror/lang-javascript';
import { rust } from '@codemirror/lang-rust';
import { json } from '@codemirror/lang-json';
import { markdown } from '@codemirror/lang-markdown';
import { css } from '@codemirror/lang-css';
import { html } from '@codemirror/lang-html';
import type { Extension } from '@codemirror/state';

export function getLanguageExtension(filename: string): Extension | null {
  const ext = filename.split('.').pop()?.toLowerCase();

  switch (ext) {
    case 'ts':
    case 'tsx':
      return javascript({ typescript: true, jsx: ext === 'tsx' });
    case 'js':
    case 'jsx':
      return javascript({ jsx: ext === 'jsx' });
    case 'rs':
      return rust();
    case 'json':
      return json();
    case 'md':
      return markdown();
    case 'css':
    case 'scss':
      return css();
    case 'html':
    case 'svelte':
      return html();
    default:
      return null;
  }
}

export function getLanguageName(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase();

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
    default:
      return 'Plain Text';
  }
}
