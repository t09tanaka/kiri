# Startup Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a segment control to StartScreen that lets users choose a command (none/claude/codex) to auto-execute in the first terminal when opening a project.

**Architecture:** Global setting stored in `kiri-settings.json` (globalSettings). StartScreen displays segment control bound to settingsStore. Terminal.svelte reads the setting and sends the command to PTY after creation, but only for the first terminal of a new window.

**Tech Stack:** Svelte 5, TypeScript, Tauri plugin-store

---

### Task 1: Add StartupCommand type to persistenceService

**Files:**
- Modify: `src/lib/services/persistenceService.ts:1-13`

**Step 1: Write the failing test**

Create test file:

```typescript
// src/lib/services/persistenceService.test.ts
import { describe, it, expect } from 'vitest';
import {
  STARTUP_COMMANDS,
  DEFAULT_STARTUP_COMMAND,
  getStartupCommandString,
  type StartupCommand,
} from './persistenceService';

describe('StartupCommand', () => {
  describe('STARTUP_COMMANDS', () => {
    it('should have three options: none, claude, codex', () => {
      expect(STARTUP_COMMANDS).toHaveLength(3);
      expect(STARTUP_COMMANDS.map((c) => c.id)).toEqual(['none', 'claude', 'codex']);
    });

    it('should have labels for each option', () => {
      expect(STARTUP_COMMANDS[0].label).toBe('None');
      expect(STARTUP_COMMANDS[1].label).toBe('Claude');
      expect(STARTUP_COMMANDS[2].label).toBe('Codex');
    });

    it('should have command strings (empty for none)', () => {
      expect(STARTUP_COMMANDS[0].command).toBe('');
      expect(STARTUP_COMMANDS[1].command).toBe('claude');
      expect(STARTUP_COMMANDS[2].command).toBe('codex');
    });
  });

  describe('DEFAULT_STARTUP_COMMAND', () => {
    it('should be none', () => {
      expect(DEFAULT_STARTUP_COMMAND).toBe('none');
    });
  });

  describe('getStartupCommandString', () => {
    it('should return empty string for none', () => {
      expect(getStartupCommandString('none')).toBe('');
    });

    it('should return claude for claude', () => {
      expect(getStartupCommandString('claude')).toBe('claude');
    });

    it('should return codex for codex', () => {
      expect(getStartupCommandString('codex')).toBe('codex');
    });

    it('should return empty string for unknown value', () => {
      expect(getStartupCommandString('unknown' as StartupCommand)).toBe('');
    });
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/services/persistenceService.test.ts`
Expected: FAIL - exports not found

**Step 3: Write implementation**

Add to `src/lib/services/persistenceService.ts`:

```typescript
// At the top of the file, after imports

export type StartupCommand = 'none' | 'claude' | 'codex';

export const DEFAULT_STARTUP_COMMAND: StartupCommand = 'none';

export interface StartupCommandOption {
  id: StartupCommand;
  label: string;
  command: string;
}

export const STARTUP_COMMANDS: StartupCommandOption[] = [
  { id: 'none', label: 'None', command: '' },
  { id: 'claude', label: 'Claude', command: 'claude' },
  { id: 'codex', label: 'Codex', command: 'codex' },
];

/**
 * Get the shell command string for a startup command setting
 */
export function getStartupCommandString(id: StartupCommand): string {
  const cmd = STARTUP_COMMANDS.find((c) => c.id === id);
  return cmd?.command ?? '';
}
```

Update `PersistedSettings` interface:

```typescript
export interface PersistedSettings {
  fontSize: number;
  startupCommand: StartupCommand;
}

const DEFAULT_SETTINGS: PersistedSettings = {
  fontSize: 13,
  startupCommand: DEFAULT_STARTUP_COMMAND,
};
```

Update `loadSettings()` to include `startupCommand`:

```typescript
return {
  fontSize: settings.fontSize ?? DEFAULT_SETTINGS.fontSize,
  startupCommand: settings.startupCommand ?? DEFAULT_SETTINGS.startupCommand,
};
```

**Step 4: Run test to verify it passes**

Run: `npm run test -- src/lib/services/persistenceService.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/services/persistenceService.ts src/lib/services/persistenceService.test.ts
git commit -m "feat(startup-command): add StartupCommand type and constants to persistenceService"
```

---

### Task 2: Add startupCommand to settingsStore

**Files:**
- Modify: `src/lib/stores/settingsStore.ts`
- Modify: `src/lib/stores/settingsStore.test.ts`

**Step 1: Write the failing tests**

Add to `src/lib/stores/settingsStore.test.ts`:

```typescript
// Add import for startupCommand
import { settingsStore, fontSize, startupCommand, FONT_SIZE_CONSTRAINTS } from './settingsStore';

// Add new describe block at the end:

describe('startupCommand', () => {
  it('should have default value of none', () => {
    const state = get(settingsStore);
    expect(state.startupCommand).toBe('none');
  });

  it('should set startup command', () => {
    settingsStore.setStartupCommand('claude');
    const state = get(settingsStore);
    expect(state.startupCommand).toBe('claude');
  });

  it('should set to codex', () => {
    settingsStore.setStartupCommand('codex');
    const state = get(settingsStore);
    expect(state.startupCommand).toBe('codex');
  });

  it('should set back to none', () => {
    settingsStore.setStartupCommand('claude');
    settingsStore.setStartupCommand('none');
    const state = get(settingsStore);
    expect(state.startupCommand).toBe('none');
  });

  it('should be included in getStateForPersistence', () => {
    settingsStore.setStartupCommand('claude');
    const state = settingsStore.getStateForPersistence();
    expect(state.startupCommand).toBe('claude');
  });

  it('should be restored from persisted state', () => {
    settingsStore.restoreState({ startupCommand: 'codex' });
    const state = get(settingsStore);
    expect(state.startupCommand).toBe('codex');
  });

  it('should default to none when not in persisted state', () => {
    settingsStore.restoreState({});
    const state = get(settingsStore);
    expect(state.startupCommand).toBe('none');
  });

  it('should reset to none on store reset', () => {
    settingsStore.setStartupCommand('claude');
    settingsStore.reset();
    const state = get(settingsStore);
    expect(state.startupCommand).toBe('none');
  });
});

describe('derived store: startupCommand', () => {
  it('should reflect current startup command', () => {
    expect(get(startupCommand)).toBe('none');
    settingsStore.setStartupCommand('claude');
    expect(get(startupCommand)).toBe('claude');
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `npm run test -- src/lib/stores/settingsStore.test.ts`
Expected: FAIL - startupCommand not exported

**Step 3: Write implementation**

Modify `src/lib/stores/settingsStore.ts`:

```typescript
import { writable, derived, get } from 'svelte/store';
import { DEFAULT_STARTUP_COMMAND, type StartupCommand } from '@/lib/services/persistenceService';

// ... existing constants ...

export interface SettingsState {
  fontSize: number;
  startupCommand: StartupCommand;
}

const initialState: SettingsState = {
  fontSize: DEFAULT_FONT_SIZE,
  startupCommand: DEFAULT_STARTUP_COMMAND,
};
```

Add `setStartupCommand` method to the store:

```typescript
setStartupCommand: (command: StartupCommand) => {
  update((state) => ({
    ...state,
    startupCommand: command,
  }));
},
```

Update `restoreState`:

```typescript
restoreState: (state: Partial<SettingsState>) => {
  update((current) => ({
    ...current,
    fontSize: state.fontSize ?? DEFAULT_FONT_SIZE,
    startupCommand: state.startupCommand ?? DEFAULT_STARTUP_COMMAND,
  }));
},
```

Add derived store:

```typescript
export const startupCommand = derived(settingsStore, ($settings) => $settings.startupCommand);
```

**Step 4: Run tests to verify they pass**

Run: `npm run test -- src/lib/stores/settingsStore.test.ts`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add src/lib/stores/settingsStore.ts src/lib/stores/settingsStore.test.ts
git commit -m "feat(startup-command): add startupCommand state to settingsStore"
```

---

### Task 3: Add segment control to StartScreen

**Files:**
- Modify: `src/lib/components/start/StartScreen.svelte`

**Step 1: Add imports and state**

In the `<script>` section of `StartScreen.svelte`, add:

```typescript
import { settingsStore, startupCommand } from '@/lib/stores/settingsStore';
import { saveSettings } from '@/lib/services/persistenceService';
import { STARTUP_COMMANDS, type StartupCommand } from '@/lib/services/persistenceService';
```

Add handler function:

```typescript
function handleStartupCommandChange(command: StartupCommand) {
  settingsStore.setStartupCommand(command);
  // Save immediately since auto-save may not be enabled on StartScreen
  saveSettings(settingsStore.getStateForPersistence());
}
```

**Step 2: Add segment control UI**

After the `<button class="open-button">` and before the `{#if $recentProjects.length > 0}`, add:

```svelte
<div class="startup-command-section">
  <span class="startup-label">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="4 17 10 11 4 5"></polyline>
      <line x1="12" y1="19" x2="20" y2="19"></line>
    </svg>
    Startup Command
  </span>
  <div class="segment-control">
    {#each STARTUP_COMMANDS as cmd (cmd.id)}
      <button
        class="segment-btn"
        class:active={$startupCommand === cmd.id}
        onclick={() => handleStartupCommandChange(cmd.id)}
      >
        {cmd.label}
      </button>
    {/each}
  </div>
</div>
```

**Step 3: Add styles**

Add to the `<style>` section:

```css
/* ===== Startup Command Section ===== */
.startup-command-section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: var(--space-4);
  padding: var(--space-3) var(--space-4);
  background: var(--bg-glass);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  transition: border-color var(--transition-normal);
}

.startup-command-section:hover {
  border-color: var(--border-glow);
}

.startup-label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
  letter-spacing: 0.02em;
  white-space: nowrap;
}

.startup-label svg {
  color: var(--accent-color);
  opacity: 0.6;
}

.segment-control {
  display: flex;
  gap: 2px;
  padding: 2px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-subtle);
}

.segment-btn {
  padding: 6px 16px;
  background: transparent;
  border: none;
  border-radius: calc(var(--radius-md) - 2px);
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}

.segment-btn:hover:not(.active) {
  color: var(--text-secondary);
  background: rgba(125, 211, 252, 0.05);
}

.segment-btn.active {
  background: var(--accent-subtle);
  color: var(--accent-color);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}
```

**Step 4: Run lint and type check**

Run: `npm run lint && npm run check`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/components/start/StartScreen.svelte
git commit -m "feat(startup-command): add segment control UI to StartScreen"
```

---

### Task 4: Execute startup command in Terminal.svelte

**Files:**
- Modify: `src/lib/components/terminal/Terminal.svelte`

**Step 1: Add imports**

Add to Terminal.svelte imports:

```typescript
import { startupCommand } from '@/lib/stores/settingsStore';
import { getStartupCommandString } from '@/lib/services/persistenceService';
```

**Step 2: Add startup command execution logic**

In the `initTerminal()` function, after the PTY is created and the terminal is registered (around line 556, after `terminalRegistry.register()`), add:

```typescript
// Execute startup command if configured
// Only for newly created terminals (not reattached from registry)
// and only for the first pane of the first tab
if (existingTerminalId === null) {
  const state = get(tabStore);
  const isFirstTab = state.tabs.length === 1;
  const firstTab = state.tabs[0];
  const isFirstPane =
    isFirstTab &&
    firstTab?.type === 'terminal' &&
    firstTab.rootPane.type === 'terminal' &&
    firstTab.rootPane.id === paneId;

  if (isFirstPane) {
    const commandStr = getStartupCommandString(get(startupCommand));
    if (commandStr) {
      // Wait for shell to be ready before sending command
      setTimeout(() => {
        if (terminalId !== null) {
          terminalService.writeTerminal(terminalId, commandStr + '\n');
        }
      }, 300);
    }
  }
}
```

**Step 3: Run lint and type check**

Run: `npm run lint && npm run check`
Expected: PASS

**Step 4: Commit**

```bash
git add src/lib/components/terminal/Terminal.svelte
git commit -m "feat(startup-command): execute startup command in first terminal on project open"
```

---

### Task 5: Run all tests and verify

**Step 1: Run all frontend tests**

Run: `npm run test`
Expected: ALL PASS

**Step 2: Run lint and type check**

Run: `npm run lint && npm run check`
Expected: PASS

**Step 3: Run Rust tests**

Run: `npm run test:rust`
Expected: PASS (no Rust changes)

**Step 4: Commit if any fixes needed**

---

### Task 6: Manual verification with Tauri dev

**Step 1: Start the app**

Run: `npm run tauri dev`

**Step 2: Verify StartScreen UI**

- Check segment control appears between "Open Directory" and Recent Projects
- Verify "None" is selected by default
- Click "Claude" and "Codex" to confirm selection works
- Verify selection persists after refreshing

**Step 3: Verify command execution**

- Select "Claude" in the segment control
- Open a project
- Verify `claude` is typed into the terminal automatically

**Step 4: Verify edge cases**

- Open additional tabs (Cmd+T) → no auto-command
- Split a pane → no auto-command on new pane
- Select "None" → open project → no auto-command
