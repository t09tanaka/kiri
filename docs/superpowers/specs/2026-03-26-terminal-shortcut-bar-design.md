# Terminal Shortcut Bar Design

## Overview

A shortcut button bar displayed at the bottom of the terminal when an AI process (Claude or Codex) is running. Provides quick access to common phrases with tap-to-send and shift+tap-to-input-only behavior. Users can customize shortcuts via a settings modal.

## Requirements

1. **Visibility**: Bar appears only when `claude` or `codex` is the foreground terminal process
2. **Default shortcuts**: OK, Continue, LGTM (3 built-in shortcuts)
3. **Click behavior**: Click sends text + Enter; Shift+Click inputs text only (no Enter)
4. **Customization**: Users can add, edit, and delete custom shortcuts via settings modal
5. **Persistence**: Shortcuts saved to project settings (`kiri-settings.json`)
6. **Design**: Follows kiri Mist theme (glass effect, slide-in animation)

## Architecture

### Components

| Component | Location | Responsibility |
|-----------|----------|----------------|
| `TerminalShortcutBar.svelte` | `src/lib/components/terminal/` | Button bar UI, click handlers |
| `TerminalShortcutSettings.svelte` | `src/lib/components/terminal/` | Settings modal for CRUD |
| `shortcutStore.ts` | `src/lib/stores/` | Shortcut state management |

### Data Flow

```
Process Detection (existing 2s polling)
    ↓
TabBar / Terminal polls getProcessName()
    ↓
shortcutStore: isAiProcessRunning derived
    ↓
TerminalShortcutBar: show/hide based on store
    ↓
User clicks button
    ↓
terminalService.writeTerminal(id, text + '\r')  // click
terminalService.writeTerminal(id, text)          // shift+click
```

### Data Structures

```typescript
interface TerminalShortcut {
  id: string;        // Unique identifier (uuid)
  label: string;     // Button display text
  text: string;      // Text to send to terminal
  builtin: boolean;  // true for default shortcuts, false for user-added
}
```

Default shortcuts:

| id | label | text | builtin |
|----|-------|------|---------|
| `builtin-ok` | OK | `OK` | true |
| `builtin-continue` | Continue | `continue` | true |
| `builtin-lgtm` | LGTM | `LGTM` | true |

### Process Detection

Leverages existing infrastructure:

- `terminalService.getProcessName(id)` returns the foreground process name
- TabBar already polls this every 2000ms
- Add a derived store `isAiProcessRunning` that checks if process name matches `claude` or `codex`
- The shortcut bar subscribes to this store for visibility

Detection targets (case-insensitive match):
- `claude` — Claude Code CLI
- `codex` — OpenAI Codex CLI

### Persistence

Stored in `kiri-settings.json` via `tauri-plugin-store`, alongside existing project settings (worktree config, etc.):

```json
{
  "shortcuts": {
    "custom": [
      {
        "id": "user-1",
        "label": "Fix",
        "text": "fix it",
        "builtin": false
      }
    ]
  }
}
```

Built-in shortcuts are hardcoded and always present. Users can only add/edit/delete custom shortcuts.

### UI Design

#### Shortcut Bar

- Position: Bottom of terminal area, above the terminal content
- Appearance: Glass effect background (`--bg-glass`, `backdrop-filter: blur(24px)`)
- Layout: Horizontal row of pill-shaped buttons, scrollable if overflow
- Animation: Slide up on appear, slide down on disappear (300ms ease)
- Right side: Gear icon button to open settings modal
- Button style: Small pill shape with `--border-glow` border, hover glow effect
- Shift indicator: When Shift is held, buttons show a subtle visual change (e.g., dashed border) to indicate "input only" mode

#### Settings Modal

- Trigger: Gear icon in shortcut bar
- Pattern: Same as WorktreePanel settings modal
- Content:
  - List of custom shortcuts (label + text) with edit/delete buttons
  - "Add shortcut" button at bottom
  - Inline editing (click to edit label/text)
- Built-in shortcuts shown but not editable/deletable (greyed out controls)

### Integration Points

#### Terminal.svelte

- Import and render `TerminalShortcutBar` at the bottom of the terminal container
- Pass `terminalId` so the bar can write to the correct terminal
- Bar needs access to the active terminal's process name

#### shortcutStore.ts

```typescript
// State
let customShortcuts = $state<TerminalShortcut[]>([]);

// Constants
const BUILTIN_SHORTCUTS: TerminalShortcut[] = [
  { id: 'builtin-ok', label: 'OK', text: 'OK', builtin: true },
  { id: 'builtin-continue', label: 'Continue', text: 'continue', builtin: true },
  { id: 'builtin-lgtm', label: 'LGTM', text: 'LGTM', builtin: true },
];

// Derived
const allShortcuts = $derived([...BUILTIN_SHORTCUTS, ...customShortcuts]);

// Actions
function addShortcut(label: string, text: string): void;
function updateShortcut(id: string, label: string, text: string): void;
function removeShortcut(id: string): void;
function loadShortcuts(): Promise<void>;  // Load from kiri-settings.json
function saveShortcuts(): Promise<void>;  // Save to kiri-settings.json
```

#### Process detection store

```typescript
// In terminalStore or a new aiProcessStore
let foregroundProcessName = $state<string>('');

const isAiProcessRunning = $derived(
  ['claude', 'codex'].includes(foregroundProcessName.toLowerCase())
);
```

## Testing

### Unit Tests

- `shortcutStore.test.ts` — CRUD operations, builtin immutability, persistence load/save
- Default shortcuts are always present and correct
- Custom shortcuts can be added, edited, deleted
- Process name matching logic (case-insensitive, exact match)

### Browser Tests

- `TerminalShortcutBar.browser.test.ts` — Rendering, click/shift+click behavior, visibility toggle
- Bar shows when `isAiProcessRunning` is true, hides when false
- Click sends text + `\r`, Shift+click sends text only
- Settings modal opens/closes correctly

## Edge Cases

1. **Multiple terminals**: Each terminal has its own process — bar visibility is per-terminal
2. **Process name variants**: Match case-insensitively (`Claude`, `CLAUDE`, `claude` all match)
3. **Rapid process switching**: Polling interval is 2s, so there may be a brief delay in show/hide
4. **Empty custom shortcuts**: Validate that label and text are non-empty before saving
5. **Long shortcut text**: Truncate display label if too long, but send full text
