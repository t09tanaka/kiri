import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { playwright } from '@vitest/browser-playwright';
import path from 'path';

export default defineConfig({
  plugins: [svelte({ hot: !process.env.VITEST })],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    projects: [
      // Unit tests - fast, run in jsdom
      {
        extends: true,
        test: {
          name: 'unit',
          globals: true,
          environment: 'jsdom',
          include: ['src/**/*.test.{js,ts}'],
          exclude: ['src/**/*.browser.test.{js,ts}'],
          setupFiles: ['./src/test/setup.ts'],
          coverage: {
            provider: 'v8',
            reporter: ['text', 'html', 'lcov'],
            reportsDirectory: './coverage',
            include: [
              'src/lib/stores/*.ts',
              'src/lib/utils/*.ts',
              'src/lib/services/*.ts',
              'src/lib/components/editor/languages.ts',
            ],
            exclude: [
              'src/**/*.{test,spec}.{js,ts}',
              'src/main.ts',
              'src/vite-env.d.ts',
              // Tauri-dependent files (require native runtime)
              'src/lib/stores/projectStore.ts',
              'src/lib/stores/searchStore.ts',
              'src/lib/services/persistenceService.ts',
              'src/lib/services/suggestService.ts',
              'src/lib/services/notificationService.ts',
            ],
          },
        },
      },
      // Browser tests - run in real browser via Playwright
      {
        extends: true,
        test: {
          name: 'browser',
          globals: true,
          include: ['src/**/*.browser.test.{js,ts}'],
          setupFiles: ['./src/test/browser-setup.ts'],
          browser: {
            enabled: true,
            provider: playwright(),
            instances: [{ browser: 'chromium' }],
            headless: true,
          },
        },
      },
    ],
  },
});
