# Shortcut Suggestions: Auto-detect repeated inputs and suggest shortcuts

## Overview

Automatically track repeated user inputs sent to Claude CLI, and suggest adding them as terminal shortcuts when usage patterns indicate they would be useful.

## Scope

- **Target**: Text sent to Claude CLI (AI process running only)
- **Not in scope**: Shell commands, non-AI terminal input

## Data Collection

### Capture Layer

- Hook into `Terminal.svelte`'s `onData` handler
- Buffer keystrokes, confirm as one "input" on `\r` (Enter)
- Only record when `isAiRunning === true`
- Ignore empty/whitespace-only inputs
- Strip ANSI escape sequences (arrow keys, Ctrl+C, etc.)

### InputRecord Structure

```typescript
interface InputRecord {
  text: string;              // Normalized text (trim + lowercase)
  rawText: string;           // Original text (for shortcut label)
  count: number;             // Total usage count
  lastUsed: number;          // Last used timestamp (Unix ms)
  firstSeen: number;         // First seen timestamp
  dismissedAt: number | null; // Dismissal timestamp (for cooldown)
}
```

### Storage

- Persisted in `kiri-settings.json` under `inputStats` key
- Max 1000 entries; evict by lowest `count` then oldest `lastUsed`
- Debounced save: 5 seconds after last `record()` call
- Save on Terminal unmount for unsaved data

## Suggestion Logic

### Conditions (ALL must be true)

1. **Frequency**: `count >= 3`
2. **Recency**: `lastUsed` within 7 days
3. **Not registered**: Text does not match any existing shortcut (built-in + custom)
4. **Not dismissed**: `dismissedAt` is null, OR cooldown period (7 days) has elapsed

### Evaluation Timing

- Checked on every `record()` call (after Enter)
- If conditions met, add to suggestion queue

### Suggestion Queue

- Max 5 suggestions
- Sorted by `count` descending
- Removed when added to shortcuts or dismissed

## Dismissal & Cooldown

- Dismissing a suggestion sets `dismissedAt` to current timestamp
- The suggestion will not reappear for 7 days
- After 7 days, if usage conditions are still met, it may be suggested again

## UI Design

### Badge (on TerminalShortcutBar)

- Displayed next to the settings gear icon
- Shows suggestion count (e.g., `+3`)
- Hidden when suggestion count is 0
- Uses accent color with subtle glow (Mist design)

### Suggestion Popover (ShortcutSuggestions.svelte)

- Opens on badge click
- Glass effect background, soft border (Mist design)
- Each row shows:
  - Input text
  - Usage count
  - Add button `[+]` → adds as shortcut, shows success toast
  - Dismiss button `[×]` → starts 7-day cooldown

### Shortcut Type Auto-detection

- Text starting with `/` → type: `'command'`
- All other text → type: `'reply'`

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `src/lib/services/inputStatsService.ts` | Record, aggregate, and evaluate suggestion logic |
| `src/lib/components/terminal/ShortcutSuggestions.svelte` | Badge + popover UI component |

### Modified Files

| File | Change |
|------|--------|
| `Terminal.svelte` | Add capture hook in onData, mount ShortcutSuggestions |
| `TerminalShortcutBar.svelte` | Add badge display area next to settings icon |
| `persistenceService.ts` | Add `loadInputStats()` / `saveInputStats()` |

## Data Flow

```
User types + Enter
  → Terminal.svelte: buffer → confirmed text
  → inputStatsService.record(text)
    → Update stats + debounced persist
    → Check suggestion conditions (excluding existing shortcuts)
    → Update suggestion queue
  → ShortcutSuggestions: reactively display badge count

User clicks [+]
  → shortcutState.addShortcut(label, text, type)
  → inputStatsService.removeSuggestion(text)
  → Toast: "Added to shortcuts"

User clicks [×]
  → inputStatsService.dismiss(text)
  → 7-day cooldown starts
```

## Testing

### Unit Tests (`inputStatsService.test.ts`)

| Test | Description |
|------|-------------|
| Basic record | Text recorded, count increments, lastUsed updates |
| Normalization | Trim + lowercase treats "  Hello " and "hello" as same |
| rawText preservation | Keeps last-used form for shortcut label |
| Suggestion conditions | 3+ count, within 7 days, not registered, not dismissed |
| Existing shortcut exclusion | Registered shortcut text is not suggested |
| Dismissal cooldown | Not suggested for 7 days after dismiss, re-suggested after |
| Entry limit | Evicts lowest count + oldest when exceeding 1000 |
| Queue limit | Max 5 suggestions, sorted by count desc |
| Type detection | `/`-prefixed → command, otherwise → reply |

### Browser Tests (`ShortcutSuggestions.browser.test.ts`)

| Test | Description |
|------|-------------|
| Badge visibility | Hidden at 0 suggestions, visible at 1+ |
| Popover toggle | Opens on click, closes on outside click |
| Add button | Fires onAdd callback |
| Dismiss button | Fires onDismiss callback |
