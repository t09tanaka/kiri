import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import prettier from 'eslint-config-prettier';
import globals from 'globals';

/** @type {import('eslint').Linter.Config[]} */
export default [
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs['flat/recommended'],
  prettier,
  ...svelte.configs['flat/prettier'],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
      },
    },
  },
  {
    files: ['**/*.svelte.ts'],
    languageOptions: {
      parser: ts.parser,
    },
  },
  // Restrict direct Tauri API imports in components
  {
    files: ['src/lib/components/**/*.svelte', 'src/lib/components/**/*.ts'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['@tauri-apps/api/*', '@tauri-apps/plugin-*'],
              message:
                'Components should not import Tauri APIs directly. Use service layer instead (src/lib/services/).',
            },
          ],
        },
      ],
    },
  },
  // Multi-window enforcement (issue #50): window-bound code must pass data via
  // URL params + Tauri events, never via shared Svelte stores. A second window
  // is a separate JS realm, so a store import here would silently desync.
  {
    files: ['src/lib/services/windowService.ts', 'src/lib/services/window*.ts'],
    ignores: ['src/lib/services/**/*.test.ts'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['@/lib/stores/*', '**/stores/*'],
              message:
                'windowService must not import shared stores. A new window runs in a separate JS realm and cannot share store state. Pass data via URL params + Tauri events instead. See docs/multi-window.md.',
            },
          ],
        },
      ],
    },
  },
  {
    rules: {
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    },
  },
  {
    ignores: [
      'src-tauri/**/*',
      'dist/**/*',
      'node_modules/**/*',
      'coverage/**/*',
      // Cargo workspace build artifacts (rustdoc, target/, etc.)
      'target/**/*',
    ],
  },
];
