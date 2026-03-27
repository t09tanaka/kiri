# Dynamic Quick Reply for AI CLI Responses

## Overview

Capture AI CLI (Claude Code / Codex) responses via Stop hooks, extract actionable choices using a lightweight LLM, and present them as numbered quick reply buttons in kiri's terminal UI. Users can select choices by pressing number keys (1-9) for instant submission.

## Architecture

```
AI CLI response complete
  -> Stop Hook (command type) fires
    -> extract-choices.sh reads response from stdin
    -> Claude env: claude --bare -p --model haiku (JSON extraction)
    -> Codex env: codex --json (JSON extraction)
    -> curl POST to kiri HTTP server (localhost:{PORT})
      -> Rust backend receives -> app.emit("claude-choices")
        -> DynamicChoicesBar renders vertically above shortcut bar
          -> Number keys (1-9) for instant send + dismiss
```

## Components

### 1. HTTP Receiver Server (Rust)

**File:** `src-tauri/src/commands/choice_server.rs`

- Starts on a dynamically allocated free port when a project opens
- Single endpoint: `POST /claude-choices`
- On receive: `app.emit("claude-choices", payload)` to forward to frontend
- Stops when kiri exits

**Port Management:**
- Each project gets its own port (no conflicts with multiple projects)
- Port number is written into hook configuration files

### 2. Hook Auto-Configuration (Rust + TypeScript)

On project open:
1. Check if `claude` CLI exists -> write Stop hook to `.claude/settings.local.json`
2. Check if `codex` CLI exists -> write Stop hook to `.codex/hooks.json`
3. If both exist, configure both
4. Embed the allocated port number in hook config

On setting toggle OFF:
- Remove hook configuration from both files
- Stop HTTP server

### 3. Hook Script

**File:** `.claude/hooks/extract-choices.sh` (shared by both CLIs)

```bash
#!/bin/bash
# Read response JSON from stdin
RESPONSE=$(cat)

# Extract response text (field name may differ by CLI)
TEXT=$(echo "$RESPONSE" | jq -r '.response_text // .result // ""')

# Skip if empty
[ -z "$TEXT" ] && exit 0

# Extract choices using AI
if command -v claude &>/dev/null; then
  CHOICES=$(echo "$TEXT" | claude --bare -p \
    "Extract choices from this text. Return JSON." \
    --model haiku \
    --output-format json \
    --json-schema '...')
elif command -v codex &>/dev/null; then
  CHOICES=$(echo "$TEXT" | codex --json \
    "Extract choices from this text. Return JSON.")
fi

# POST to kiri
curl -s -X POST "http://localhost:${KIRI_PORT}/claude-choices" \
  -H "Content-Type: application/json" \
  -d "$CHOICES" &
```

### 4. Extraction Schema

```json
{
  "type": "object",
  "properties": {
    "choices": {
      "type": "array",
      "items": { "type": "string" },
      "maxItems": 9
    },
    "type": {
      "enum": ["options", "yes_no", "none"]
    }
  },
  "required": ["choices", "type"]
}
```

- `type: "options"` — Numbered list choices (e.g., "1. Do X", "2. Do Y")
- `type: "yes_no"` — Confirmation questions (e.g., "Proceed? (y/n)")
- `type: "none"` — No actionable choices detected; do not display

### 5. DynamicChoicesBar (Svelte)

**File:** `src/lib/components/terminal/DynamicChoicesBar.svelte`

**Layout:** Vertical list above the existing TerminalShortcutBar

```
+-------------------------------+
| 1  xterm.js buffer approach   |
| 2  PTY output in Rust         |
| 3  Claude Code CLI option     |
+-------------------------------+
| REPLY | yes | no | ...        |  <- existing TerminalShortcutBar
| CMD   | /commit | ...         |
```

**Behavior:**
- Visible only when: AI process is running AND choices are available
- Number keys (1-9): instant send (write text + Enter to terminal) + dismiss
- slideIn animation on appear (consistent with existing shortcut bar)
- Choices are dismissed after selection

**Styling:**
- Follows kiri's Mist design concept
- Subtle glass effect background
- Number badges with accent color
- Vertical layout with clear visual hierarchy

### 6. Settings

**New field in shortcut settings:**
- `dynamicChoices: boolean` (default: `true`)
- Toggle added to TerminalShortcutSettings modal
- Label: "Dynamic Quick Reply" with description

**Behavior on toggle:**
- ON: Configure hooks + start HTTP server
- OFF: Remove hook config + stop HTTP server

## Detection Patterns

The LLM extraction prompt targets two patterns:

### Numbered Options
AI presents multiple choices as a numbered or lettered list:
```
1. Use approach A
2. Use approach B
3. Use approach C
```

### Yes/No Confirmation
AI asks a confirmation question:
```
Should I proceed with this change?
Do you want me to fix this?
```

## Edge Cases

| Case | Handling |
|------|----------|
| claude/codex not installed | Skip hook configuration, feature disabled |
| Haiku/GPT extraction fails | Silent failure, no quick reply shown |
| Multiple projects open | Each project uses its own port |
| kiri exits | HTTP server stops, hook POST silently fails |
| No choices detected (`type: "none"`) | Bar hidden |
| More than 9 choices | Show first 9 only |
| Rapid consecutive responses | New choices overwrite previous |
| Hook script not executable | Auto-set chmod +x on creation |
| Unknown stdin JSON field name | Check actual payload format from each CLI at implementation time |

## File Changes Summary

### New Files
- `src-tauri/src/commands/choice_server.rs` — HTTP receiver server
- `src/lib/components/terminal/DynamicChoicesBar.svelte` — UI component
- `.claude/hooks/extract-choices.sh` — Hook script (auto-generated per project)

### Modified Files
- `src-tauri/src/commands/mod.rs` — Register choice_server module
- `src-tauri/src/lib.rs` — Register new Tauri commands
- `src/lib/components/terminal/Terminal.svelte` — Integrate DynamicChoicesBar
- `src/lib/stores/shortcutStore.svelte.ts` — Add `dynamicChoices` setting
- `src/lib/services/persistenceService.ts` — Persist dynamicChoices setting
- `src/lib/components/terminal/TerminalShortcutSettings.svelte` — Add toggle
