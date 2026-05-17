# Internal Drag & Drop (File Move) Design

## Goal

Enable drag-and-drop file/directory movement within the file explorer tree.

## Decisions

- **Move only** (no copy via modifier key)
- **Single file/directory** at a time (no multi-select)
- **Invalid targets disabled during drag** (not post-drop error)
- **Mouse events** (not HTML Drag API, which may conflict with Tauri's webview)

## Architecture

### Event Source Separation

| Source | Events | Action |
|--------|--------|--------|
| External D&D (OS files) | `tauri://drag-*` | Copy |
| Internal D&D (tree items) | `mousedown`/`mousemove`/`mouseup` | Move |

### Reused Infrastructure

- `dragDropStore` - `startDrag`/`endDrag`/`setDropTarget`
- `resolveDropTarget()` - Drop target resolution
- `data-drop-path`/`data-drop-is-dir` attributes - Target identification
- `elementFromPoint` - Hit testing
- Drop target CSS (`.drop-target`, `.drop-target-child`)
- Auto-expand (`drag-auto-expand`)

### New Components

**Backend (Rust):**
- `move_path(source, target_dir)` command
- `fs::rename` with cross-device fallback (copy + delete)
- Reuse existing name deduplication logic

**Frontend:**
- `fileService.movePath()` - Service method
- `dragDropService.moveToDirectory()` - D&D service method
- `isDescendantOf()` - Validation utility
- FileTreeItem: `mousedown` handler (drag initiation)
- FileTree: `mousemove`/`mouseup` handlers, floating ghost element

## Interaction Flow

### Drag Start
- `mousedown` on FileTreeItem + 5px movement threshold
- `dragDropStore.startDrag([path])` updates store
- `.dragging-source` CSS class on source item (semi-transparent)
- Floating ghost element near cursor (icon + filename)

### During Drag
- `mousemove` -> `elementFromPoint` -> `resolveDropTarget` -> store update
- Existing highlight and auto-expand work automatically
- Invalid targets (same directory, descendant of dragged directory) -> `dropTargetPath = null` + forbidden icon on ghost

### Drop
- `mouseup` with valid `dropTargetPath` -> `moveToDirectory()`
- Success: File tree auto-refreshes via watcher
- Failure: Toast error notification

### Cancel
- `Escape` key cancels drag
- `mouseup` outside file tree cancels drag

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/commands/fs.rs` | Add `move_path` command |
| `src/lib/services/fileService.ts` | Add `movePath()` |
| `src/lib/services/dragDropService.ts` | Add `moveToDirectory()` |
| `src/lib/utils/dragDrop.ts` | Add `isDescendantOf()` |
| `src/lib/components/filetree/FileTreeItem.svelte` | Add `mousedown` handler |
| `src/lib/components/filetree/FileTree.svelte` | Add `mousemove`/`mouseup`, ghost element |

## Tests

- Rust: `move_path` unit tests (normal move, cross-device, name conflict, circular reference rejection)
- Frontend: `isDescendantOf()` utility tests
- Frontend: `dragDropService.moveToDirectory()` tests
