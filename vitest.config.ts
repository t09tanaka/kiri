import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte({ hot: !process.env.VITEST })],
  test: {
    globals: true,
    environment: 'jsdom',
    include: ['src/**/*.{test,spec}.{js,ts}'],
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
      ],
    },
  },
});
