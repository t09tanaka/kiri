# Multi-window data passing

A second Tauri window runs in its **own JS realm**: it has its own module
graph, its own Svelte component instances, and its own store singletons. A
shared `writable` looks identical on both sides — but mutations on window A
never reach window B.

## Rule

`windowService` and any other code that **opens, signals, or hydrates a
window from the parent side** must not import shared Svelte stores. Pass
data across windows via:

1. **URL search params** — for initial state the child window needs to
   hydrate from on boot (e.g. `?projectPath=/foo`).
2. **Tauri events** (`@tauri-apps/api/event`) — for ongoing updates or
   commands directed at a specific window.

## Enforcement

This rule is checked by ESLint via `no-restricted-imports` on
`src/lib/services/windowService.ts` and any sibling `window*.ts` file. The
override lives in `eslint.config.js`. Tests are exempt.

```js
{
  files: ['src/lib/services/windowService.ts', 'src/lib/services/window*.ts'],
  ignores: ['src/lib/services/**/*.test.ts'],
  rules: {
    'no-restricted-imports': ['error', { patterns: [{
      group: ['@/lib/stores/*', '**/stores/*'],
      message: '...',
    }] }],
  },
}
```

## What this catches

```ts
// ❌ Caught — window-bound code reading from a parent-window store
import { projectStore } from '@/lib/stores/projectStore';
export async function openProjectWindow() {
  const path = get(projectStore).currentPath; // child window would never see this
  await getCurrentWindow().emit('hydrate', { path });
}
```

```ts
// ✅ Allowed — caller passes the value explicitly, windowService stays store-free
export async function openProjectWindow(projectPath: string) {
  const url = new URL('/project', window.location.origin);
  url.searchParams.set('path', projectPath);
  await WebviewWindow.getByLabel('project') ?? new WebviewWindow('project', { url: url.href });
}
```
