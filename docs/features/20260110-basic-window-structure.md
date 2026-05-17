# Basic Window Structure Design

## Overview

Design for the basic window layout of Kiri application.

## Layout Structure

```
┌─────────────────────────────────────────────────────────┐
│                      Title Bar                          │
├──────────────┬──────────────────────────────────────────┤
│              │                                          │
│   Sidebar    │              Main Content                │
│  (File Tree) │         (Terminal / Editor)              │
│              │                                          │
│   200-300px  │              flexible                    │
│   resizable  │                                          │
│              │                                          │
├──────────────┴──────────────────────────────────────────┤
│                      Status Bar                         │
└─────────────────────────────────────────────────────────┘
```

## Components

### 1. AppLayout.svelte
- Root layout component
- CSS Grid-based layout
- Manages overall structure

### 2. Sidebar.svelte
- Left panel for file tree
- Width: 200px default, resizable (150-400px)
- Contains: FileTree component (future)

### 3. MainContent.svelte
- Main content area
- Displays Terminal or Editor based on current mode
- Fills remaining space

### 4. StatusBar.svelte
- Bottom status bar
- Height: 24px fixed
- Shows: current mode, file info, etc.

## CSS Variables

```css
:root {
  /* Colors */
  --bg-primary: #1e1e1e;
  --bg-secondary: #252526;
  --bg-tertiary: #2d2d2d;
  --text-primary: #cccccc;
  --text-secondary: #808080;
  --border-color: #3c3c3c;
  --accent-color: #0078d4;

  /* Sizing */
  --sidebar-width: 200px;
  --sidebar-min-width: 150px;
  --sidebar-max-width: 400px;
  --statusbar-height: 24px;
  --titlebar-height: 0px; /* Native title bar */
}
```

## File Structure

```
src/
├── lib/
│   ├── components/
│   │   ├── layout/
│   │   │   ├── AppLayout.svelte
│   │   │   ├── Sidebar.svelte
│   │   │   ├── MainContent.svelte
│   │   │   └── StatusBar.svelte
│   │   └── index.ts
│   └── stores/
│       └── appStore.ts
├── App.svelte
├── app.css
└── main.ts
```

## State Management

### appStore.ts
```typescript
interface AppState {
  sidebarWidth: number;
  currentMode: 'terminal' | 'editor';
  currentFile: string | null;
}
```

## Implementation Steps

1. Create CSS variables in app.css
2. Create AppLayout.svelte with grid layout
3. Create Sidebar.svelte (empty placeholder)
4. Create MainContent.svelte (empty placeholder)
5. Create StatusBar.svelte
6. Create appStore.ts for state management
7. Update App.svelte to use AppLayout

## Acceptance Criteria

- [ ] Window displays with sidebar + main content layout
- [ ] Sidebar is visible with placeholder content
- [ ] Main content area fills remaining space
- [ ] Status bar shows at bottom
- [ ] Dark theme applied
- [ ] Responsive to window resize
