# Mode Switching Design

## Overview

Design for switching between Terminal and Editor modes.

## Requirements

1. Click on file in tree -> switch to Editor mode
2. Click on Terminal button/tab -> switch to Terminal mode
3. Keyboard shortcut to toggle modes
4. Status bar shows current mode

## UI Design

### Option 1: Tabs in Header

```
┌─────────────────────────────────────────────────────────┐
│  [Terminal] [filename.ts]                    [x]        │
├─────────────────────────────────────────────────────────┤
│                    Content Area                         │
└─────────────────────────────────────────────────────────┘
```

### Option 2: Mode Toggle Button (Selected)

Add a toggle button in the header to switch between modes.
Simpler for v0.0.1, can expand to tabs in v0.0.2.

```
┌─────────────────────────────────────────────────────────┐
│  TERMINAL  [Toggle] | filename.ts                       │
├─────────────────────────────────────────────────────────┤
│                    Content Area                         │
└─────────────────────────────────────────────────────────┘
```

## Keyboard Shortcuts

- `Ctrl+\`` (or Cmd+\`) - Toggle Terminal/Editor
- Clicking file in tree - Opens in Editor
- Clicking Terminal in status bar - Switch to Terminal

## Implementation

### Update StatusBar

Add clickable mode indicator that toggles mode.

### Update MainContent Header

Add toggle button to switch modes.

### Add Keyboard Shortcuts

Listen for keyboard events to toggle modes.

## State Management

Already implemented in appStore:

- currentMode: 'terminal' | 'editor'
- setMode(mode)

## Implementation Steps

1. Add toggle button to MainContent header
2. Make StatusBar mode clickable
3. Add keyboard shortcut handler
4. Ensure mode persists correctly

## Acceptance Criteria

- [ ] Can click toggle button to switch modes
- [ ] Can click file to open in editor
- [ ] Status bar shows current mode
- [ ] Keyboard shortcut works (Ctrl/Cmd + `)
