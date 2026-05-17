import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

// Bundle-splitting strategy
// =========================
//
// The bundle is split via a mix of `manualChunks` here and dynamic
// `import()` call sites in the source. Run `npm run build &&
// npm run perf:bundle-report` to see every emitted chunk and which
// category it falls into (vendor / modal / codemirror-lang /
// xterm-addon / app-or-route).
//
// - `vendor-codemirror-core`: the CodeMirror core packages every
//   editor mount needs. Pulled out so QuickOpen / Editor / EditorModal
//   share one cached chunk after the first load.
// - `vendor-xterm`: the four xterm modules. Terminal.svelte awaits
//   core + addon-fit synchronously (see issue #36); WebLinks / Canvas
//   are deferred to after the first paint via a separate
//   `await import()`, so they reach the user only when terminal
//   output starts arriving.
// - Modals (ContentSearchModal, DiffViewModal, …) — App.svelte loads
//   each via `() => import('…')` factories, so they emit per-modal
//   chunks (see issue #33).
// - Editor language packs — `getLanguageExtension` keeps eight
//   `import('@codemirror/lang-*')` call sites; ts/tsx/js/jsx share
//   one chunk (see issue #32).
//
// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          // Core CodeMirror (shared by all editor instances).
          'vendor-codemirror-core': [
            '@codemirror/state',
            '@codemirror/view',
            '@codemirror/commands',
            '@codemirror/language',
            '@lezer/highlight',
          ],
          // Terminal core + addons (loaded lazily by Terminal.svelte).
          'vendor-xterm': [
            '@xterm/xterm',
            '@xterm/addon-fit',
            '@xterm/addon-web-links',
            '@xterm/addon-canvas',
          ],
        },
      },
    },
  },
});
