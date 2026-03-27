# Dynamic Quick Reply Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture AI CLI responses via Stop hooks, extract choices with a lightweight LLM, and present them as numbered quick reply buttons in kiri's terminal.

**Architecture:** HTTP receiver server in Rust receives extracted choices from a Stop hook script, emits Tauri events to the frontend, which renders a vertical DynamicChoicesBar above the existing TerminalShortcutBar. Number keys 1-9 instantly send the choice and dismiss the bar.

**Tech Stack:** Rust (axum for HTTP server), Svelte 5, TypeScript, Claude Code hooks, Codex hooks

---

## File Structure

### New Files
| File | Responsibility |
|------|----------------|
| `src-tauri/src/commands/choice_server.rs` | HTTP server: start/stop, receive choices, emit Tauri events |
| `src-tauri/src/commands/choice_server_commands.rs` | Tauri command wrappers for choice server |
| `src/lib/components/terminal/DynamicChoicesBar.svelte` | UI: vertical numbered choice list with keyboard handling |
| `src/lib/services/choiceServerService.ts` | Frontend service: Tauri command wrappers for choice server |
| `src/lib/services/hookConfigService.ts` | Frontend service: write/remove hook configs for Claude/Codex |

### Modified Files
| File | Change |
|------|--------|
| `src-tauri/src/commands/mod.rs` | Register new modules |
| `src-tauri/src/lib.rs` | Register new commands, manage server state |
| `src/lib/components/terminal/Terminal.svelte` | Integrate DynamicChoicesBar, add number key handling |
| `src/lib/stores/shortcutStore.svelte.ts` | Add `dynamicChoices` boolean setting |
| `src/lib/services/persistenceService.ts` | Add load/save for `dynamicChoices` setting |
| `src/lib/components/terminal/TerminalShortcutSettings.svelte` | Add Dynamic Quick Reply toggle |

---

## Task 1: Choice Server — Pure Logic (Rust)

**Files:**
- Create: `src-tauri/src/commands/choice_server.rs`

This task implements the core HTTP server logic without Tauri dependencies.

- [ ] **Step 1: Write the failing test for ChoicePayload deserialization**

```rust
// src-tauri/src/commands/choice_server.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceType {
    Options,
    YesNo,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoicePayload {
    pub choices: Vec<String>,
    #[serde(rename = "type")]
    pub choice_type: ChoiceType,
}

impl ChoicePayload {
    /// Returns true if there are actionable choices to display
    pub fn has_choices(&self) -> bool {
        self.choice_type != ChoiceType::None && !self.choices.is_empty()
    }

    /// Returns at most 9 choices (keyboard limit 1-9)
    pub fn capped_choices(&self) -> Vec<String> {
        self.choices.iter().take(9).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_options_payload() {
        let json = r#"{"choices":["Use approach A","Use approach B"],"type":"options"}"#;
        let payload: ChoicePayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.choice_type, ChoiceType::Options);
        assert_eq!(payload.choices.len(), 2);
        assert_eq!(payload.choices[0], "Use approach A");
    }

    #[test]
    fn test_deserialize_yes_no_payload() {
        let json = r#"{"choices":["yes","no"],"type":"yes_no"}"#;
        let payload: ChoicePayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.choice_type, ChoiceType::YesNo);
        assert_eq!(payload.choices.len(), 2);
    }

    #[test]
    fn test_deserialize_none_payload() {
        let json = r#"{"choices":[],"type":"none"}"#;
        let payload: ChoicePayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.choice_type, ChoiceType::None);
        assert!(payload.choices.is_empty());
    }

    #[test]
    fn test_has_choices() {
        let with = ChoicePayload {
            choices: vec!["a".into()],
            choice_type: ChoiceType::Options,
        };
        assert!(with.has_choices());

        let none = ChoicePayload {
            choices: vec![],
            choice_type: ChoiceType::None,
        };
        assert!(!none.has_choices());

        let empty_options = ChoicePayload {
            choices: vec![],
            choice_type: ChoiceType::Options,
        };
        assert!(!empty_options.has_choices());
    }

    #[test]
    fn test_capped_choices_under_limit() {
        let payload = ChoicePayload {
            choices: vec!["a".into(), "b".into(), "c".into()],
            choice_type: ChoiceType::Options,
        };
        assert_eq!(payload.capped_choices().len(), 3);
    }

    #[test]
    fn test_capped_choices_over_limit() {
        let choices: Vec<String> = (1..=12).map(|i| format!("choice {}", i)).collect();
        let payload = ChoicePayload {
            choices,
            choice_type: ChoiceType::Options,
        };
        let capped = payload.capped_choices();
        assert_eq!(capped.len(), 9);
        assert_eq!(capped[8], "choice 9");
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cd src-tauri && cargo test choice_server -- --nocapture`
Expected: All 6 tests pass

- [ ] **Step 3: Add find_available_port function with test**

Add to the same file:

```rust
use std::net::TcpListener;

/// Find an available TCP port on localhost
pub fn find_available_port() -> Result<u16, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

#[cfg(test)]
mod tests {
    // ... existing tests ...

    #[test]
    fn test_find_available_port() {
        let port = find_available_port().unwrap();
        assert!(port > 0);
        // Verify the port is actually available by binding to it
        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", port));
        assert!(listener.is_ok());
    }
}
```

- [ ] **Step 4: Run all tests**

Run: `cd src-tauri && cargo test choice_server -- --nocapture`
Expected: All 7 tests pass

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/choice_server.rs
git commit -m "feat(choice-server): add ChoicePayload types and port allocation"
```

---

## Task 2: Choice Server — Axum HTTP Server (Rust)

**Files:**
- Modify: `src-tauri/src/commands/choice_server.rs`

Add the axum HTTP server that receives choices and emits Tauri events.

- [ ] **Step 1: Add HTTP server implementation**

Add to `choice_server.rs`:

```rust
use axum::{extract::State as AxumState, http::StatusCode, routing::post, Json, Router};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// State shared with axum route handlers
struct ServerState {
    app: AppHandle,
}

/// Tauri event payload for frontend
#[derive(Debug, Clone, Serialize)]
pub struct ChoiceEvent {
    pub choices: Vec<String>,
    pub choice_type: String,
}

/// POST /claude-choices handler
async fn handle_choices(
    AxumState(state): AxumState<Arc<ServerState>>,
    Json(payload): Json<ChoicePayload>,
) -> StatusCode {
    if !payload.has_choices() {
        return StatusCode::OK;
    }

    let event = ChoiceEvent {
        choices: payload.capped_choices(),
        choice_type: match payload.choice_type {
            ChoiceType::Options => "options".to_string(),
            ChoiceType::YesNo => "yes_no".to_string(),
            ChoiceType::None => "none".to_string(),
        },
    };

    match state.app.emit("claude-choices", &event) {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Handle for a running choice server
pub struct ChoiceServerHandle {
    pub port: u16,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl ChoiceServerHandle {
    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Managed state for Tauri
pub type ChoiceServerState = Arc<Mutex<Option<ChoiceServerHandle>>>;

/// Start the choice HTTP server on an available port
pub async fn start_server(app: AppHandle) -> Result<ChoiceServerHandle, String> {
    let port = find_available_port().map_err(|e| e.to_string())?;

    let state = Arc::new(ServerState { app });
    let router = Router::new()
        .route("/claude-choices", post(handle_choices))
        .with_state(state);

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| e.to_string())?;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .ok();
    });

    Ok(ChoiceServerHandle { port, shutdown_tx })
}
```

- [ ] **Step 2: Run existing tests to verify no regressions**

Run: `cd src-tauri && cargo test choice_server -- --nocapture`
Expected: All 7 existing tests still pass (new code has no unit tests as it requires Tauri runtime)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/choice_server.rs
git commit -m "feat(choice-server): add axum HTTP server with graceful shutdown"
```

---

## Task 3: Choice Server — Tauri Command Wrappers

**Files:**
- Create: `src-tauri/src/commands/choice_server_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create command wrappers**

```rust
// src-tauri/src/commands/choice_server_commands.rs

use crate::commands::choice_server::{self, ChoiceServerState};
use tauri::AppHandle;

#[tauri::command]
pub async fn start_choice_server(
    app: AppHandle,
    state: tauri::State<'_, ChoiceServerState>,
) -> Result<u16, String> {
    let mut server = state.lock().await;
    if let Some(handle) = server.as_ref() {
        return Ok(handle.port);
    }
    let handle = choice_server::start_server(app).await?;
    let port = handle.port;
    *server = Some(handle);
    Ok(port)
}

#[tauri::command]
pub async fn stop_choice_server(
    state: tauri::State<'_, ChoiceServerState>,
) -> Result<(), String> {
    let mut server = state.lock().await;
    if let Some(handle) = server.take() {
        handle.shutdown();
    }
    Ok(())
}

#[tauri::command]
pub async fn get_choice_server_port(
    state: tauri::State<'_, ChoiceServerState>,
) -> Result<Option<u16>, String> {
    let server = state.lock().await;
    Ok(server.as_ref().map(|h| h.port))
}
```

- [ ] **Step 2: Register modules in mod.rs**

Add to `src-tauri/src/commands/mod.rs`:

```rust
// After existing module declarations:
pub mod choice_server;
pub mod choice_server_commands;

// After existing pub use statements:
pub use choice_server::ChoiceServerState;
pub use choice_server_commands::*;
```

- [ ] **Step 3: Register commands and state in lib.rs**

In `src-tauri/src/lib.rs`, add to imports:

```rust
use commands::{
    // ... existing imports ...
    start_choice_server, stop_choice_server, get_choice_server_port,
    ChoiceServerState,
};
```

Add state management after existing `.manage()` calls:

```rust
.manage(Arc::new(tokio::sync::Mutex::new(None::<commands::choice_server::ChoiceServerHandle>)) as ChoiceServerState)
```

Add commands to invoke_handler:

```rust
// Choice server
start_choice_server,
stop_choice_server,
get_choice_server_port,
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo build`
Expected: Compiles without errors

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/choice_server_commands.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(choice-server): add Tauri command wrappers and register server state"
```

---

## Task 4: Frontend Service — Choice Server

**Files:**
- Create: `src/lib/services/choiceServerService.ts`

- [ ] **Step 1: Create the service**

```typescript
// src/lib/services/choiceServerService.ts
import { invoke } from '@tauri-apps/api/core';

export const choiceServerService = {
  /** Start the choice HTTP server and return the port */
  start: (): Promise<number> => invoke('start_choice_server'),

  /** Stop the choice HTTP server */
  stop: (): Promise<void> => invoke('stop_choice_server'),

  /** Get the current server port (null if not running) */
  getPort: (): Promise<number | null> => invoke('get_choice_server_port'),
};
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/services/choiceServerService.ts
git commit -m "feat(choice-server): add frontend service for choice server commands"
```

---

## Task 5: Hook Configuration Service

**Files:**
- Create: `src/lib/services/hookConfigService.ts`
- Test: `src/lib/services/hookConfigService.test.ts`

- [ ] **Step 1: Write failing tests for hook config generation**

```typescript
// src/lib/services/hookConfigService.test.ts
import { describe, expect, it } from 'vitest';
import {
  generateClaudeHookConfig,
  generateCodexHookConfig,
  generateHookScript,
} from './hookConfigService';

describe('generateHookScript', () => {
  it('generates a script with the given port', () => {
    const script = generateHookScript(12345);
    expect(script).toContain('KIRI_PORT=12345');
    expect(script).toContain('#!/bin/bash');
    expect(script).toContain('claude --bare');
    expect(script).toContain('codex');
    expect(script).toContain('curl');
  });
});

describe('generateClaudeHookConfig', () => {
  it('generates a settings.local.json Stop hook entry', () => {
    const config = generateClaudeHookConfig('/path/to/hook.sh');
    expect(config.hooks).toBeDefined();
    expect(config.hooks.Stop).toBeDefined();
    expect(config.hooks.Stop[0].type).toBe('command');
    expect(config.hooks.Stop[0].command).toBe('/path/to/hook.sh');
  });

  it('sets async to true', () => {
    const config = generateClaudeHookConfig('/path/to/hook.sh');
    expect(config.hooks.Stop[0].async).toBe(true);
  });
});

describe('generateCodexHookConfig', () => {
  it('generates a hooks.json Stop hook entry', () => {
    const config = generateCodexHookConfig('/path/to/hook.sh');
    expect(config).toHaveLength(1);
    expect(config[0].event).toBe('Stop');
    expect(config[0].command).toContain('/path/to/hook.sh');
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test -- --run hookConfigService`
Expected: FAIL — module not found

- [ ] **Step 3: Implement the service (pure logic functions only)**

```typescript
// src/lib/services/hookConfigService.ts

/**
 * Generate the hook shell script content
 */
export function generateHookScript(port: number): string {
  return `#!/bin/bash
# Auto-generated by kiri — Dynamic Quick Reply hook
# Extracts choices from AI CLI responses and sends to kiri

KIRI_PORT=${port}
RESPONSE=$(cat)

# Extract the stop_ts response text from hook payload
# Claude Code provides: { "session_id": "...", "result": "...", ... }
# Codex provides a similar structure
TEXT=$(echo "$RESPONSE" | jq -r '.result // .response_text // .message // ""' 2>/dev/null)

# Skip if empty or too short
[ -z "$TEXT" ] || [ \${#TEXT} -lt 10 ] && exit 0

SCHEMA='{"type":"object","properties":{"choices":{"type":"array","items":{"type":"string"},"maxItems":9},"type":{"enum":["options","yes_no","none"]}},"required":["choices","type"]}'

PROMPT="Analyze the following AI assistant response and extract any actionable choices the user needs to pick from.

Rules:
- If the response presents numbered/lettered options (like 1. X, 2. Y, A) X, B) Y), extract each option text (without the number/letter prefix) as type \\"options\\"
- If the response asks a yes/no or confirmation question, return [\\"yes\\", \\"no\\"] as type \\"yes_no\\"
- If there are no choices to make, return type \\"none\\" with empty choices array
- Keep choice text concise (under 80 chars each)
- Maximum 9 choices

Response to analyze:
\$TEXT_CONTENT"

TEXT_CONTENT=$(echo "$TEXT" | tail -c 4000)
FULL_PROMPT=$(echo "$PROMPT" | sed "s/\\\$TEXT_CONTENT/$TEXT_CONTENT/")

# Try claude first, fall back to codex
if command -v claude &>/dev/null; then
  CHOICES=$(echo "$FULL_PROMPT" | claude --bare -p --model haiku --output-format json --json-schema "$SCHEMA" 2>/dev/null | jq -r '.result // .' 2>/dev/null)
elif command -v codex &>/dev/null; then
  CHOICES=$(echo "$TEXT_CONTENT" | codex -p "Extract choices as JSON with fields: choices (string array) and type (options|yes_no|none)" --json 2>/dev/null)
fi

# Skip if extraction failed
[ -z "$CHOICES" ] && exit 0

# POST to kiri (fire and forget)
curl -s -X POST "http://127.0.0.1:\${KIRI_PORT}/claude-choices" \\
  -H "Content-Type: application/json" \\
  -d "$CHOICES" &>/dev/null &

exit 0
`;
}

/**
 * Generate Claude Code hook config for .claude/settings.local.json
 */
export function generateClaudeHookConfig(hookScriptPath: string): {
  hooks: { Stop: Array<{ type: string; command: string; async: boolean; timeout: number }> };
} {
  return {
    hooks: {
      Stop: [
        {
          type: 'command',
          command: hookScriptPath,
          async: true,
          timeout: 30,
        },
      ],
    },
  };
}

/**
 * Generate Codex hook config for .codex/hooks.json
 */
export function generateCodexHookConfig(
  hookScriptPath: string
): Array<{ event: string; command: string }> {
  return [
    {
      event: 'Stop',
      command: hookScriptPath,
    },
  ];
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test -- --run hookConfigService`
Expected: All 5 tests pass

- [ ] **Step 5: Add file I/O functions (Tauri-dependent, not unit tested)**

Add to `hookConfigService.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';
import { writeTextFile, readTextFile, mkdir, exists } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';

/**
 * Check if a CLI command exists on the system
 */
async function commandExists(command: string): Promise<boolean> {
  try {
    const result = await invoke<{ code: number }>('run_init_command', {
      cwd: '/',
      command: `command -v ${command}`,
    });
    return result.code === 0;
  } catch {
    return false;
  }
}

/**
 * Configure hooks for a project directory
 * Returns the server port if hooks were configured, null otherwise
 */
export async function configureHooks(projectPath: string, port: number): Promise<boolean> {
  const hookScript = generateHookScript(port);
  const hookDir = await join(projectPath, '.claude', 'hooks');
  const hookScriptPath = await join(hookDir, 'kiri-extract-choices.sh');

  // Write hook script
  if (!(await exists(hookDir))) {
    await mkdir(hookDir, { recursive: true });
  }
  await writeTextFile(hookScriptPath, hookScript);

  // Make executable
  await invoke('run_init_command', {
    cwd: projectPath,
    command: `chmod +x "${hookScriptPath}"`,
  });

  let configured = false;

  // Configure Claude Code hooks
  if (await commandExists('claude')) {
    const claudeDir = await join(projectPath, '.claude');
    const claudeSettingsPath = await join(claudeDir, 'settings.local.json');

    let settings: Record<string, unknown> = {};
    try {
      const content = await readTextFile(claudeSettingsPath);
      settings = JSON.parse(content);
    } catch {
      // File doesn't exist or invalid JSON
    }

    const hookConfig = generateClaudeHookConfig(hookScriptPath);

    // Merge hook config (preserve existing hooks for other events)
    if (!settings.hooks || typeof settings.hooks !== 'object') {
      settings.hooks = {};
    }
    (settings.hooks as Record<string, unknown>).Stop = hookConfig.hooks.Stop;

    await writeTextFile(claudeSettingsPath, JSON.stringify(settings, null, 2));
    configured = true;
  }

  // Configure Codex hooks
  if (await commandExists('codex')) {
    const codexDir = await join(projectPath, '.codex');
    const codexHooksPath = await join(codexDir, 'hooks.json');

    if (!(await exists(codexDir))) {
      await mkdir(codexDir, { recursive: true });
    }

    let hooks: Array<Record<string, unknown>> = [];
    try {
      const content = await readTextFile(codexHooksPath);
      hooks = JSON.parse(content);
    } catch {
      // File doesn't exist or invalid JSON
    }

    // Remove existing kiri hooks, add new one
    hooks = hooks.filter((h) => !(h.command as string)?.includes('kiri-extract-choices'));
    const codexConfig = generateCodexHookConfig(hookScriptPath);
    hooks.push(...codexConfig);

    await writeTextFile(codexHooksPath, JSON.stringify(hooks, null, 2));
    configured = true;
  }

  return configured;
}

/**
 * Remove hook configuration from a project
 */
export async function removeHooks(projectPath: string): Promise<void> {
  // Remove from Claude Code settings.local.json
  try {
    const claudeSettingsPath = await join(projectPath, '.claude', 'settings.local.json');
    const content = await readTextFile(claudeSettingsPath);
    const settings = JSON.parse(content);
    if (settings.hooks?.Stop) {
      settings.hooks.Stop = (settings.hooks.Stop as Array<Record<string, unknown>>).filter(
        (h) => !(h.command as string)?.includes('kiri-extract-choices')
      );
      if (settings.hooks.Stop.length === 0) {
        delete settings.hooks.Stop;
      }
      if (Object.keys(settings.hooks).length === 0) {
        delete settings.hooks;
      }
      await writeTextFile(claudeSettingsPath, JSON.stringify(settings, null, 2));
    }
  } catch {
    // File doesn't exist
  }

  // Remove from Codex hooks.json
  try {
    const codexHooksPath = await join(projectPath, '.codex', 'hooks.json');
    const content = await readTextFile(codexHooksPath);
    let hooks: Array<Record<string, unknown>> = JSON.parse(content);
    hooks = hooks.filter((h) => !(h.command as string)?.includes('kiri-extract-choices'));
    await writeTextFile(codexHooksPath, JSON.stringify(hooks, null, 2));
  } catch {
    // File doesn't exist
  }
}
```

- [ ] **Step 6: Commit**

```bash
git add src/lib/services/hookConfigService.ts src/lib/services/hookConfigService.test.ts
git commit -m "feat(hooks): add hook configuration service for Claude/Codex integration"
```

---

## Task 6: Settings — dynamicChoices Persistence

**Files:**
- Modify: `src/lib/stores/shortcutStore.svelte.ts`
- Modify: `src/lib/services/persistenceService.ts`
- Test: `src/lib/stores/shortcutStore.test.ts` (verify existing tests still pass)

- [ ] **Step 1: Add dynamicChoices to ShortcutState**

In `src/lib/stores/shortcutStore.svelte.ts`, add to the `ShortcutState` class:

```typescript
class ShortcutState {
  customShortcuts = $state<TerminalShortcut[]>([]);
  dynamicChoices = $state<boolean>(true);

  // ... existing methods ...
}
```

Also add to `createShortcutStore()`:

```typescript
export function createShortcutStore() {
  let customShortcuts: TerminalShortcut[] = [];
  let dynamicChoices: boolean = true;

  return {
    // ... existing methods ...

    getDynamicChoices(): boolean {
      return dynamicChoices;
    },

    setDynamicChoices(value: boolean): void {
      dynamicChoices = value;
    },
  };
}
```

- [ ] **Step 2: Add persistence functions**

In `src/lib/services/persistenceService.ts`, add at the end:

```typescript
// ============================================================================
// Dynamic Quick Reply Setting
// ============================================================================

/**
 * Load dynamic choices setting
 */
export async function loadDynamicChoices(): Promise<boolean> {
  try {
    const s = await getStore();
    await s.reload();
    const value = await s.get<boolean>('dynamicChoices');
    return value ?? true; // default ON
  } catch (error) {
    console.error('Failed to load dynamicChoices:', error);
    return true;
  }
}

/**
 * Save dynamic choices setting
 */
export async function saveDynamicChoices(enabled: boolean): Promise<void> {
  try {
    const s = await getStore();
    await s.set('dynamicChoices', enabled);
    await s.save();
  } catch (error) {
    console.error('Failed to save dynamicChoices:', error);
  }
}
```

- [ ] **Step 3: Run existing tests to verify no regressions**

Run: `npm run test -- --run shortcutStore`
Expected: All existing tests pass

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/shortcutStore.svelte.ts src/lib/services/persistenceService.ts
git commit -m "feat(settings): add dynamicChoices setting with persistence"
```

---

## Task 7: DynamicChoicesBar UI Component

**Files:**
- Create: `src/lib/components/terminal/DynamicChoicesBar.svelte`

- [ ] **Step 1: Create the component**

```svelte
<!-- src/lib/components/terminal/DynamicChoicesBar.svelte -->
<script lang="ts">
  interface Choice {
    index: number;
    text: string;
  }

  interface Props {
    choices: string[];
    onSelect: (text: string) => void;
  }

  let { choices, onSelect }: Props = $props();

  const items: Choice[] = $derived(
    choices.map((text, i) => ({ index: i + 1, text }))
  );

  function handleClick(choice: Choice) {
    onSelect(choice.text);
  }
</script>

{#if choices.length > 0}
  <div class="dynamic-choices">
    {#each items as choice (choice.index)}
      <button
        class="choice-row"
        onclick={() => handleClick(choice)}
        title="Press {choice.index} to select"
      >
        <span class="choice-badge">{choice.index}</span>
        <span class="choice-text">{choice.text}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .dynamic-choices {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px var(--space-3);
    background: linear-gradient(
      180deg,
      rgba(252, 211, 77, 0.06) 0%,
      rgba(13, 17, 23, 0.85) 100%
    );
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border-top: 1px solid rgba(252, 211, 77, 0.2);
    animation: choicesSlideUp 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes choicesSlideUp {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .choice-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 4px 8px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    transition:
      background var(--transition-fast),
      border-color var(--transition-fast);
  }

  .choice-row:hover {
    background: rgba(252, 211, 77, 0.08);
    border-color: rgba(252, 211, 77, 0.2);
  }

  .choice-row:active {
    transform: scale(0.99);
  }

  .choice-badge {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    color: var(--accent3-color, #fcd34d);
    background: rgba(252, 211, 77, 0.12);
    border: 1px solid rgba(252, 211, 77, 0.3);
    border-radius: 6px;
    text-shadow: 0 0 8px rgba(252, 211, 77, 0.3);
  }

  .choice-text {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/terminal/DynamicChoicesBar.svelte
git commit -m "feat(ui): add DynamicChoicesBar component with vertical layout"
```

---

## Task 8: Integrate DynamicChoicesBar into Terminal

**Files:**
- Modify: `src/lib/components/terminal/Terminal.svelte`

- [ ] **Step 1: Add imports, state, and event listener**

In Terminal.svelte `<script>` section, add imports:

```typescript
import DynamicChoicesBar from './DynamicChoicesBar.svelte';
import { choiceServerService } from '@/lib/services/choiceServerService';
import { loadDynamicChoices } from '@/lib/services/persistenceService';
```

Add state variables (near existing terminal state):

```typescript
let dynamicChoices = $state<string[]>([]);
```

In the `onMount` / `initTerminal` area, add event listener for choices:

```typescript
// In onMount, after existing event listeners:
let unlistenChoices: (() => void) | null = null;

// Listen for claude-choices events
eventService.listen<{ choices: string[]; choice_type: string }>('claude-choices', (event) => {
  dynamicChoices = event.payload.choices;
}).then((fn) => {
  unlistenChoices = fn;
});
```

In cleanup/onDestroy, add:

```typescript
unlistenChoices?.();
```

- [ ] **Step 2: Add number key handler**

In the keyboard event handler (capture phase on textarea, around line 651), add before existing handlers:

```typescript
// Handle number keys for dynamic choices (1-9)
if (
  dynamicChoices.length > 0 &&
  isAiRunning &&
  !event.metaKey &&
  !event.ctrlKey &&
  !event.altKey &&
  event.key >= '1' &&
  event.key <= '9'
) {
  const index = parseInt(event.key) - 1;
  if (index < dynamicChoices.length) {
    event.preventDefault();
    event.stopPropagation();
    handleChoiceSelect(dynamicChoices[index]);
    return;
  }
}
```

Add the handler function:

```typescript
function handleChoiceSelect(text: string) {
  if (terminalId === null) return;
  terminalService.writeTerminal(terminalId, text + '\r');
  dynamicChoices = [];
  terminal?.focus();
}
```

- [ ] **Step 3: Add DynamicChoicesBar to template**

Right before `<TerminalShortcutBar>` (around line 979), add:

```svelte
<DynamicChoicesBar
  choices={isAiRunning ? dynamicChoices : []}
  onSelect={handleChoiceSelect}
/>
```

- [ ] **Step 4: Verify compilation**

Run: `npm run check`
Expected: No TypeScript errors

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/terminal/Terminal.svelte
git commit -m "feat(terminal): integrate DynamicChoicesBar with keyboard shortcuts"
```

---

## Task 9: Settings Toggle in TerminalShortcutSettings

**Files:**
- Modify: `src/lib/components/terminal/TerminalShortcutSettings.svelte`
- Modify: `src/lib/components/terminal/Terminal.svelte`

- [ ] **Step 1: Add toggle prop and UI to TerminalShortcutSettings**

Add to Props interface:

```typescript
interface Props {
  // ... existing props ...
  dynamicChoicesEnabled: boolean;
  onDynamicChoicesToggle: (enabled: boolean) => void;
}
```

Add to destructuring:

```typescript
let {
  // ... existing ...
  dynamicChoicesEnabled,
  onDynamicChoicesToggle,
}: Props = $props();
```

Add toggle UI in the modal-content, between the shortcut-list and modal-footer:

```svelte
<!-- Dynamic Quick Reply toggle -->
<div class="dynamic-choices-section">
  <div class="toggle-row">
    <div class="toggle-info">
      <span class="toggle-label">Dynamic Quick Reply</span>
      <span class="toggle-desc">Auto-detect choices from AI responses</span>
    </div>
    <button
      class="toggle-btn"
      class:active={dynamicChoicesEnabled}
      onclick={() => onDynamicChoicesToggle(!dynamicChoicesEnabled)}
      role="switch"
      aria-checked={dynamicChoicesEnabled}
      aria-label="Toggle Dynamic Quick Reply"
    >
      <span class="toggle-track">
        <span class="toggle-thumb"></span>
      </span>
    </button>
  </div>
</div>
```

Add styles:

```css
.dynamic-choices-section {
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--border-subtle);
}

.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}

.toggle-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.toggle-label {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 500;
  color: var(--text-primary);
}

.toggle-desc {
  font-size: 11px;
  color: var(--text-muted);
}

.toggle-btn {
  flex-shrink: 0;
  padding: 2px;
  background: transparent;
  border: none;
  cursor: pointer;
}

.toggle-track {
  display: flex;
  align-items: center;
  width: 36px;
  height: 20px;
  padding: 2px;
  background: rgba(125, 211, 252, 0.1);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  transition:
    background var(--transition-fast),
    border-color var(--transition-fast);
}

.toggle-btn.active .toggle-track {
  background: rgba(125, 211, 252, 0.25);
  border-color: rgba(125, 211, 252, 0.4);
}

.toggle-thumb {
  width: 14px;
  height: 14px;
  background: var(--text-muted);
  border-radius: 50%;
  transition:
    transform var(--transition-fast),
    background var(--transition-fast);
}

.toggle-btn.active .toggle-thumb {
  transform: translateX(16px);
  background: var(--accent-color);
  box-shadow: 0 0 8px rgba(125, 211, 252, 0.3);
}
```

- [ ] **Step 2: Wire up in Terminal.svelte**

Add new state and handler in Terminal.svelte:

```typescript
let dynamicChoicesEnabled = $state(true);

// In onMount:
dynamicChoicesEnabled = await loadDynamicChoices();
```

Update `<TerminalShortcutSettings>` usage:

```svelte
<TerminalShortcutSettings
  open={showShortcutSettings}
  shortcuts={shortcutState.allShortcuts}
  focusSection={shortcutFocusSection}
  dynamicChoicesEnabled={dynamicChoicesEnabled}
  onDynamicChoicesToggle={handleDynamicChoicesToggle}
  onClose={() => { ... }}
  onAdd={handleShortcutAdd}
  onUpdate={handleShortcutUpdate}
  onRemove={handleShortcutRemove}
/>
```

Add the toggle handler:

```typescript
import { saveDynamicChoices } from '@/lib/services/persistenceService';
import { configureHooks, removeHooks } from '@/lib/services/hookConfigService';

async function handleDynamicChoicesToggle(enabled: boolean) {
  dynamicChoicesEnabled = enabled;
  await saveDynamicChoices(enabled);

  // Get project path from URL params
  const params = new URLSearchParams(window.location.search);
  const projectPath = params.get('path');
  if (!projectPath) return;

  if (enabled) {
    try {
      const port = await choiceServerService.start();
      await configureHooks(projectPath, port);
    } catch (error) {
      console.error('Failed to configure hooks:', error);
    }
  } else {
    try {
      await removeHooks(projectPath);
      await choiceServerService.stop();
      dynamicChoices = [];
    } catch (error) {
      console.error('Failed to remove hooks:', error);
    }
  }
}
```

- [ ] **Step 3: Verify compilation**

Run: `npm run check`
Expected: No TypeScript errors

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/terminal/TerminalShortcutSettings.svelte src/lib/components/terminal/Terminal.svelte
git commit -m "feat(settings): add Dynamic Quick Reply toggle to shortcut settings"
```

---

## Task 10: Auto-Configure on Project Open

**Files:**
- Modify: `src/lib/components/terminal/Terminal.svelte`

- [ ] **Step 1: Add auto-configuration in onMount**

In Terminal.svelte's `onMount`, after loading shortcuts and dynamic choices setting:

```typescript
// Auto-configure hooks if dynamic choices is enabled
if (dynamicChoicesEnabled) {
  const params = new URLSearchParams(window.location.search);
  const projectPath = params.get('path');
  if (projectPath) {
    try {
      const port = await choiceServerService.start();
      await configureHooks(projectPath, port);
    } catch (error) {
      console.error('Failed to auto-configure choice hooks:', error);
    }
  }
}
```

- [ ] **Step 2: Add cleanup on unmount**

In the cleanup section (onDestroy or return from onMount):

```typescript
// Stop choice server on cleanup
choiceServerService.stop().catch(() => {});
```

- [ ] **Step 3: Verify compilation**

Run: `npm run check`
Expected: No TypeScript errors

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/terminal/Terminal.svelte
git commit -m "feat(terminal): auto-configure hooks on project open"
```

---

## Task 11: Tests — DynamicChoicesBar Browser Test

**Files:**
- Create: `src/lib/components/terminal/DynamicChoicesBar.browser.test.ts`

- [ ] **Step 1: Write browser tests**

```typescript
// src/lib/components/terminal/DynamicChoicesBar.browser.test.ts
import { render } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import DynamicChoicesBar from './DynamicChoicesBar.svelte';

test('renders nothing when choices is empty', () => {
  const { container } = render(DynamicChoicesBar, {
    props: { choices: [], onSelect: vi.fn() },
  });
  expect(container.querySelector('.dynamic-choices')).toBeNull();
});

test('renders choices with numbered badges', () => {
  const { getAllByRole } = render(DynamicChoicesBar, {
    props: {
      choices: ['Option A', 'Option B', 'Option C'],
      onSelect: vi.fn(),
    },
  });
  const buttons = getAllByRole('button');
  expect(buttons).toHaveLength(3);
  expect(buttons[0]).toHaveTextContent('1');
  expect(buttons[0]).toHaveTextContent('Option A');
  expect(buttons[2]).toHaveTextContent('3');
  expect(buttons[2]).toHaveTextContent('Option C');
});

test('calls onSelect when a choice is clicked', async () => {
  const onSelect = vi.fn();
  const { getAllByRole } = render(DynamicChoicesBar, {
    props: {
      choices: ['Option A', 'Option B'],
      onSelect,
    },
  });
  const buttons = getAllByRole('button');
  await buttons[1].click();
  expect(onSelect).toHaveBeenCalledWith('Option B');
});
```

- [ ] **Step 2: Run browser tests**

Run: `npm run test:browser -- --run DynamicChoicesBar`
Expected: All 3 tests pass

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/terminal/DynamicChoicesBar.browser.test.ts
git commit -m "test(ui): add browser tests for DynamicChoicesBar"
```

---

## Task 12: Full Integration Verification

- [ ] **Step 1: Run all frontend tests**

Run: `npm run test`
Expected: All tests pass

- [ ] **Step 2: Run all Rust tests**

Run: `npm run test:rust`
Expected: All tests pass

- [ ] **Step 3: Run lint and type check**

Run: `npm run lint && npm run check`
Expected: No errors

- [ ] **Step 4: Build verification**

Run: `cd src-tauri && cargo build`
Expected: Compiles without errors

- [ ] **Step 5: Manual verification with `npm run tauri dev`**

1. Start kiri with `npm run tauri dev`
2. Open a project directory
3. Verify shortcut settings modal has "Dynamic Quick Reply" toggle
4. Verify `.claude/settings.local.json` has Stop hook configured (if claude CLI exists)
5. Verify `.claude/hooks/kiri-extract-choices.sh` was created and is executable
6. Open terminal, run Claude Code, and verify choices appear after a response with options
7. Verify pressing number keys sends the choice and dismisses the bar

- [ ] **Step 6: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: address integration issues from verification"
```
