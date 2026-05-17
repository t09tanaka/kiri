# Internal Drag & Drop (File Move) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable drag-and-drop file/directory movement within the file explorer tree using mouse events.

**Architecture:** Mouse events (`mousedown`/`mousemove`/`mouseup`) detect internal drags. The existing drag-drop infrastructure (`dragDropStore`, `resolveDropTarget`, `elementFromPoint`, `data-drop-path` attributes, highlight CSS, auto-expand) is fully reused. A new Rust `move_path` command handles the actual file system move operation.

**Tech Stack:** Svelte 5, TypeScript, Rust, Tauri 2.x

---

### Task 1: Rust `move_path` command

**Files:**
- Modify: `src-tauri/src/commands/drag_drop.rs`
- Modify: `src-tauri/src/commands/mod.rs` (export)
- Modify: `src-tauri/src/lib.rs` (register)

**Context:** The existing `drag_drop.rs` has `copy_paths_to_directory` and `generate_unique_name`. We add `move_path` in the same file, reusing `generate_unique_name` for name conflicts.

**Step 1: Write the failing tests**

Add to `src-tauri/src/commands/drag_drop.rs` in the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn test_move_path_file() {
    let source_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();

    let source_file = source_dir.path().join("test.txt");
    fs::write(&source_file, "content").unwrap();

    let result = move_path(
        source_file.to_string_lossy().to_string(),
        target_dir.path().to_string_lossy().to_string(),
    );

    assert!(result.is_ok());
    assert!(!source_file.exists()); // Source removed
    assert!(target_dir.path().join("test.txt").exists()); // Moved to target
}

#[test]
fn test_move_path_directory() {
    let source_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();

    let subdir = source_dir.path().join("mydir");
    fs::create_dir(&subdir).unwrap();
    fs::write(subdir.join("file.txt"), "content").unwrap();

    let result = move_path(
        subdir.to_string_lossy().to_string(),
        target_dir.path().to_string_lossy().to_string(),
    );

    assert!(result.is_ok());
    assert!(!subdir.exists());
    assert!(target_dir.path().join("mydir").exists());
    assert!(target_dir.path().join("mydir").join("file.txt").exists());
}

#[test]
fn test_move_path_name_conflict() {
    let source_dir = tempdir().unwrap();
    let target_dir = tempdir().unwrap();

    fs::write(source_dir.path().join("test.txt"), "source").unwrap();
    fs::write(target_dir.path().join("test.txt"), "existing").unwrap();

    let result = move_path(
        source_dir.path().join("test.txt").to_string_lossy().to_string(),
        target_dir.path().to_string_lossy().to_string(),
    );

    assert!(result.is_ok());
    let moved_path = result.unwrap();
    assert!(moved_path.ends_with("test (1).txt"));
    assert!(target_dir.path().join("test.txt").exists()); // Original untouched
    assert!(target_dir.path().join("test (1).txt").exists()); // Moved with new name
}

#[test]
fn test_move_path_into_descendant_rejected() {
    let dir = tempdir().unwrap();

    let parent = dir.path().join("parent");
    let child = parent.join("child");
    fs::create_dir_all(&child).unwrap();

    let result = move_path(
        parent.to_string_lossy().to_string(),
        child.to_string_lossy().to_string(),
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot move"));
    assert!(parent.exists()); // Not moved
}

#[test]
fn test_move_path_to_same_directory_rejected() {
    let dir = tempdir().unwrap();

    let file = dir.path().join("test.txt");
    fs::write(&file, "content").unwrap();

    let result = move_path(
        file.to_string_lossy().to_string(),
        dir.path().to_string_lossy().to_string(),
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already in"));
}

#[test]
fn test_move_path_nonexistent_source() {
    let target_dir = tempdir().unwrap();

    let result = move_path(
        "/nonexistent/file.txt".to_string(),
        target_dir.path().to_string_lossy().to_string(),
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

#[test]
fn test_move_path_nonexistent_target() {
    let source_dir = tempdir().unwrap();
    fs::write(source_dir.path().join("test.txt"), "content").unwrap();

    let result = move_path(
        source_dir.path().join("test.txt").to_string_lossy().to_string(),
        "/nonexistent/target".to_string(),
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test move_path -- --nocapture`
Expected: FAIL (function `move_path` not found)

**Step 3: Implement `move_path`**

Add to `src-tauri/src/commands/drag_drop.rs` before the `#[cfg(test)]` block:

```rust
/// Move a file or directory to a target directory.
/// Returns the final path of the moved item.
#[tauri::command]
pub fn move_path(source: String, target_dir: String) -> Result<String, String> {
    let source_path = Path::new(&source);
    let target_path = Path::new(&target_dir);

    // Validate source exists
    if !source_path.exists() {
        return Err(format!("Source does not exist: {}", source));
    }

    // Validate target directory exists and is a directory
    if !target_path.exists() {
        return Err(format!("Target directory does not exist: {}", target_dir));
    }
    if !target_path.is_dir() {
        return Err(format!("Target is not a directory: {}", target_dir));
    }

    // Prevent moving to same directory
    if let Some(parent) = source_path.parent() {
        if parent == target_path {
            return Err(format!(
                "Item is already in target directory: {}",
                source
            ));
        }
    }

    // Prevent moving directory into its own descendant
    if source_path.is_dir() {
        let canonical_source = source_path
            .canonicalize()
            .map_err(|e| format!("Failed to resolve source path: {}", e))?;
        let canonical_target = target_path
            .canonicalize()
            .map_err(|e| format!("Failed to resolve target path: {}", e))?;

        if canonical_target.starts_with(&canonical_source) {
            return Err(format!(
                "Cannot move directory into its own subdirectory: {} -> {}",
                source, target_dir
            ));
        }
    }

    // Generate unique name if conflict exists
    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid file name".to_string())?;

    let unique_name = generate_unique_name(file_name, target_path);
    let dest_path = target_path.join(&unique_name);

    // Try fs::rename first (fast, same filesystem)
    // Fall back to copy + delete for cross-device moves
    match std::fs::rename(source_path, &dest_path) {
        Ok(()) => Ok(dest_path.to_string_lossy().to_string()),
        Err(rename_err) => {
            // Cross-device move: copy then delete
            if source_path.is_dir() {
                copy_directory(source_path, target_path)
                    .and_then(|copied_path| {
                        std::fs::remove_dir_all(source_path)
                            .map_err(|e| format!("Copied but failed to remove source: {}", e))?;
                        Ok(copied_path)
                    })
                    .map_err(|e| format!("Move failed (rename: {}, copy: {})", rename_err, e))
            } else {
                copy_file(source_path, target_path)
                    .and_then(|copied_path| {
                        std::fs::remove_file(source_path)
                            .map_err(|e| format!("Copied but failed to remove source: {}", e))?;
                        Ok(copied_path)
                    })
                    .map_err(|e| format!("Move failed (rename: {}, copy: {})", rename_err, e))
            }
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test move_path -- --nocapture`
Expected: All 7 new tests PASS

**Step 5: Register the command**

In `src-tauri/src/commands/mod.rs`, add `move_path` to the public exports (same line as `copy_paths_to_directory`).

In `src-tauri/src/lib.rs`:
- Add `move_path` to the `use commands::{...}` import block
- Add `move_path,` after `copy_paths_to_directory,` in the `invoke_handler`

**Step 6: Run all Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS

**Step 7: Commit**

```bash
git add src-tauri/src/commands/drag_drop.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(drag-drop): add move_path command for internal file move"
```

---

### Task 2: `isDescendantOf` validation utility

**Files:**
- Modify: `src/lib/utils/dragDrop.ts`
- Modify: `src/lib/utils/dragDrop.test.ts`

**Context:** We need a function to check if a path is a descendant of another path, used to prevent circular directory moves and same-directory moves.

**Step 1: Write the failing tests**

Add to `src/lib/utils/dragDrop.test.ts`:

```typescript
import { getParentDirectory, resolveDropTarget, isDescendantOf } from './dragDrop';

describe('isDescendantOf', () => {
  it('returns true for direct child', () => {
    expect(isDescendantOf('/a/b', '/a')).toBe(true);
  });

  it('returns true for deeply nested descendant', () => {
    expect(isDescendantOf('/a/b/c/d', '/a')).toBe(true);
  });

  it('returns false for same path', () => {
    expect(isDescendantOf('/a', '/a')).toBe(false);
  });

  it('returns false for parent', () => {
    expect(isDescendantOf('/a', '/a/b')).toBe(false);
  });

  it('returns false for sibling', () => {
    expect(isDescendantOf('/a/c', '/a/b')).toBe(false);
  });

  it('returns false for prefix-similar paths', () => {
    expect(isDescendantOf('/project-backup/file', '/project')).toBe(false);
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `npm run test -- --run src/lib/utils/dragDrop.test.ts`
Expected: FAIL (isDescendantOf is not exported)

**Step 3: Implement `isDescendantOf`**

Add to `src/lib/utils/dragDrop.ts`:

```typescript
/**
 * Check if targetPath is a descendant of ancestorPath (not equal).
 * Handles prefix-similar paths correctly (e.g., /project vs /project-backup).
 */
export function isDescendantOf(targetPath: string, ancestorPath: string): boolean {
  if (targetPath === ancestorPath) return false;
  return targetPath.startsWith(ancestorPath + '/');
}
```

**Step 4: Run tests to verify they pass**

Run: `npm run test -- --run src/lib/utils/dragDrop.test.ts`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/lib/utils/dragDrop.ts src/lib/utils/dragDrop.test.ts
git commit -m "feat(drag-drop): add isDescendantOf validation utility"
```

---

### Task 3: Frontend service layer (`movePath` / `moveToDirectory`)

**Files:**
- Modify: `src/lib/services/fileService.ts`
- Modify: `src/lib/services/dragDropService.ts`

**Context:** Add `movePath` to `fileService` (raw Tauri command wrapper) and `moveToDirectory` to `dragDropService` (with validation).

**Step 1: Add `movePath` to `fileService.ts`**

Add after the `deletePath` method:

```typescript
  /**
   * Move file or directory to target directory
   * @returns Final path of the moved item
   */
  movePath: (source: string, targetDir: string): Promise<string> =>
    invoke('move_path', { source, targetDir }),
```

**Step 2: Add `moveToDirectory` to `dragDropService.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface CopyResult {
  success: boolean;
  copied: string[];
  errors: CopyError[];
}

export interface CopyError {
  path: string;
  error: string;
}

/**
 * Drag and drop service
 * Wraps Tauri commands for file copy/move operations
 */
export const dragDropService = {
  /**
   * Copy files/directories to a target directory
   */
  copyToDirectory: (sourcePaths: string[], targetDir: string): Promise<CopyResult> =>
    invoke('copy_paths_to_directory', {
      sourcePaths,
      targetDir,
    }),

  /**
   * Move a file/directory to a target directory
   * @returns Final path of the moved item
   */
  moveToDirectory: (sourcePath: string, targetDir: string): Promise<string> =>
    invoke('move_path', {
      source: sourcePath,
      targetDir,
    }),
};
```

**Step 3: Run type check**

Run: `npm run check`
Expected: 0 errors

**Step 4: Commit**

```bash
git add src/lib/services/fileService.ts src/lib/services/dragDropService.ts
git commit -m "feat(drag-drop): add moveToDirectory service method"
```

---

### Task 4: Internal drag initiation in FileTreeItem

**Files:**
- Modify: `src/lib/components/filetree/FileTreeItem.svelte`

**Context:** Add `mousedown` handler on FileTreeItem that stores the initial mouse position and path. The actual drag start is detected in FileTree when mouse moves 5px+ from the start position. We must avoid interfering with existing click/toggle behavior.

**Step 1: Add mousedown handler to FileTreeItem**

In the `<script>` section, add a function that dispatches a custom event with the entry path and mouse position:

```typescript
function handleInternalDragStart(event: MouseEvent) {
  // Only left mouse button
  if (event.button !== 0) return;
  // Don't start drag on context menu or keyboard modifiers
  if (event.ctrlKey || event.metaKey || event.shiftKey) return;

  // Dispatch custom event for FileTree to handle drag detection
  window.dispatchEvent(
    new CustomEvent('filetree-mousedown', {
      detail: {
        path: entry.path,
        isDir: entry.is_dir,
        startX: event.clientX,
        startY: event.clientY,
      },
    })
  );
}
```

On the `.tree-item` button element, add `onmousedown={handleInternalDragStart}`.

**Step 2: Add `.dragging-source` CSS class**

Add to FileTreeItem's `<style>`:

```css
.tree-item-container.dragging-source {
  opacity: 0.4;
}
```

Apply via `class:dragging-source={$isDragging && $draggedPaths.includes(entry.path)}` on the `.tree-item-container` div.

**Step 3: Run type check and lint**

Run: `npm run check && npm run lint`
Expected: 0 errors

**Step 4: Commit**

```bash
git add src/lib/components/filetree/FileTreeItem.svelte
git commit -m "feat(drag-drop): add internal drag initiation to FileTreeItem"
```

---

### Task 5: Internal drag handling in FileTree (mousemove/mouseup/ghost)

**Files:**
- Modify: `src/lib/components/filetree/FileTree.svelte`
- Modify: `src/lib/utils/dragDrop.ts` (import `isDescendantOf`)

**Context:** This is the main task. FileTree listens for `filetree-mousedown` events, tracks mouse movement, and when 5px threshold is exceeded, enters internal drag mode. Uses existing `elementFromPoint` + `resolveDropTarget` + `dragDropStore` infrastructure. Adds a floating ghost element and drop validation.

**Step 1: Add internal drag state variables**

In the `<script>` section, add after existing drag state:

```typescript
// Internal drag state (mouse-based, for moving files within tree)
let internalDragSource: { path: string; isDir: boolean } | null = null;
let internalDragStartPos: { x: number; y: number } | null = null;
let isInternalDragging = false;
const DRAG_THRESHOLD = 5; // pixels
let ghostElement: HTMLDivElement | null = null;
```

**Step 2: Add `filetree-mousedown` listener**

In `onMount`, add:

```typescript
window.addEventListener('filetree-mousedown', handleInternalMouseDown as EventListener);
```

In `onDestroy`, add:

```typescript
window.removeEventListener('filetree-mousedown', handleInternalMouseDown as EventListener);
```

Implement the handler:

```typescript
function handleInternalMouseDown(event: CustomEvent) {
  const { path, isDir, startX, startY } = event.detail;
  internalDragSource = { path, isDir };
  internalDragStartPos = { x: startX, y: startY };
  isInternalDragging = false;

  // Add temporary document-level listeners
  document.addEventListener('mousemove', handleInternalMouseMove);
  document.addEventListener('mouseup', handleInternalMouseUp);
}
```

**Step 3: Implement `handleInternalMouseMove`**

```typescript
function handleInternalMouseMove(event: MouseEvent) {
  if (!internalDragSource || !internalDragStartPos) return;

  const dx = event.clientX - internalDragStartPos.x;
  const dy = event.clientY - internalDragStartPos.y;

  // Check threshold before starting drag
  if (!isInternalDragging) {
    if (Math.sqrt(dx * dx + dy * dy) < DRAG_THRESHOLD) return;
    // Start internal drag
    isInternalDragging = true;
    dragDropStore.startDrag([internalDragSource.path]);
    createGhostElement(internalDragSource.path);
  }

  // Update ghost position
  updateGhostPosition(event.clientX, event.clientY);

  // Resolve drop target using existing infrastructure
  const element = document.elementFromPoint(event.clientX, event.clientY);
  if (!element) {
    dragDropStore.setDropTarget(null);
    handleAutoExpandOnTargetChange(null);
    updateGhostValidity(false);
    return;
  }

  const treeItem = element.closest('[data-drop-path]') as HTMLElement | null;
  if (!treeItem) {
    dragDropStore.setDropTarget(null);
    handleAutoExpandOnTargetChange(null);
    updateGhostValidity(false);
    return;
  }

  const path = treeItem.dataset.dropPath ?? null;
  const isDir = treeItem.dataset.dropIsDir === 'true';
  const targetDir = resolveDropTarget(path, isDir, rootPath);

  // Validate: cannot move to same directory or into descendant
  const isValid = isValidMoveTarget(targetDir);

  if (isValid) {
    dragDropStore.setDropTarget(targetDir);
    handleAutoExpandOnTargetChange(targetDir);
  } else {
    dragDropStore.setDropTarget(null);
    handleAutoExpandOnTargetChange(null);
  }
  updateGhostValidity(isValid);
}
```

**Step 4: Implement validation and ghost helpers**

```typescript
import { resolveDropTarget, getParentDirectory, isDescendantOf } from '@/lib/utils/dragDrop';

function isValidMoveTarget(targetDir: string | null): boolean {
  if (!targetDir || !internalDragSource) return false;
  const sourcePath = internalDragSource.path;

  // Cannot move to same directory
  const sourceParent = getParentDirectory(sourcePath);
  if (targetDir === sourceParent) return false;

  // Cannot move directory into its own descendant
  if (internalDragSource.isDir && isDescendantOf(targetDir, sourcePath)) return false;

  // Cannot move to self
  if (targetDir === sourcePath) return false;

  return true;
}

function createGhostElement(sourcePath: string) {
  const name = sourcePath.split('/').pop() || sourcePath;
  ghostElement = document.createElement('div');
  ghostElement.className = 'drag-ghost';
  ghostElement.textContent = name;
  document.body.appendChild(ghostElement);
}

function updateGhostPosition(x: number, y: number) {
  if (!ghostElement) return;
  ghostElement.style.left = `${x + 12}px`;
  ghostElement.style.top = `${y - 10}px`;
}

function updateGhostValidity(valid: boolean) {
  if (!ghostElement) return;
  ghostElement.classList.toggle('invalid', !valid);
}

function removeGhostElement() {
  if (ghostElement) {
    ghostElement.remove();
    ghostElement = null;
  }
}
```

**Step 5: Implement `handleInternalMouseUp`**

```typescript
async function handleInternalMouseUp(event: MouseEvent) {
  document.removeEventListener('mousemove', handleInternalMouseMove);
  document.removeEventListener('mouseup', handleInternalMouseUp);

  if (!isInternalDragging || !internalDragSource) {
    cleanupInternalDrag();
    return;
  }

  const targetDir = $dropTargetPath;
  const sourcePath = internalDragSource.path;

  // Only move if target is valid
  if (targetDir && isValidMoveTarget(targetDir)) {
    try {
      await dragDropService.moveToDirectory(sourcePath, targetDir);
    } catch (e) {
      toastStore.error(`Move failed: ${String(e)}`);
    }
  }

  cleanupInternalDrag();
}

function cleanupInternalDrag() {
  removeGhostElement();
  dragDropStore.endDrag();
  internalDragSource = null;
  internalDragStartPos = null;
  isInternalDragging = false;
}
```

**Step 6: Add Escape key handler**

In `onMount`, add:

```typescript
window.addEventListener('keydown', handleDragKeyDown);
```

In `onDestroy`, add:

```typescript
window.removeEventListener('keydown', handleDragKeyDown);
```

```typescript
function handleDragKeyDown(event: KeyboardEvent) {
  if (event.key === 'Escape' && isInternalDragging) {
    document.removeEventListener('mousemove', handleInternalMouseMove);
    document.removeEventListener('mouseup', handleInternalMouseUp);
    cleanupInternalDrag();
  }
}
```

**Step 7: Add ghost element CSS**

Add to `<style>` section (global styles since ghost is appended to body):

```css
:global(.drag-ghost) {
  position: fixed;
  pointer-events: none;
  z-index: 10000;
  padding: 4px 8px;
  background: var(--bg-tertiary, #1c2333);
  border: 1px solid var(--border-glow, rgba(125, 211, 252, 0.3));
  border-radius: 4px;
  color: var(--text-primary, #e6edf3);
  font-size: 12px;
  font-family: var(--font-mono, 'IBM Plex Mono', monospace);
  white-space: nowrap;
  opacity: 0.9;
}

:global(.drag-ghost.invalid) {
  border-color: var(--text-error, #f87171);
  opacity: 0.5;
}
```

**Step 8: Run type check and lint**

Run: `npm run check && npm run lint`
Expected: 0 errors

**Step 9: Run all tests**

Run: `npm run test`
Expected: All tests PASS

**Step 10: Commit**

```bash
git add src/lib/components/filetree/FileTree.svelte src/lib/components/filetree/FileTreeItem.svelte
git commit -m "feat(drag-drop): add internal drag-and-drop file move with mouse events"
```

---

### Task 6: Integration testing and edge cases

**Files:**
- Modify: `src/lib/components/filetree/FileTree.svelte` (if fixes needed)
- Modify: `src/lib/components/filetree/FileTreeItem.svelte` (if fixes needed)

**Step 1: Run all tests**

Run: `npm run test && cd src-tauri && cargo test`
Expected: All tests PASS

**Step 2: Run type check and lint**

Run: `npm run check && npm run lint`
Expected: 0 errors

**Step 3: Start app and verify**

Run: `npm run tauri dev`

Manual verification checklist:
1. Drag a file to a directory → file moves
2. Drag a directory to another directory → directory moves
3. Drag to same directory → no action (no highlight)
4. Drag directory to its own child → no action (no highlight)
5. Ghost element follows cursor with filename
6. Ghost shows invalid state for forbidden targets
7. Source item appears semi-transparent during drag
8. Drop target directory highlights blue
9. Auto-expand works when hovering directory for 2 seconds
10. Escape key cancels drag
11. External D&D from OS still works (copy, not move)
12. Click still works normally (no accidental drags)

**Step 4: Fix any issues found**

**Step 5: Final commit (if fixes needed)**

```bash
git add -u
git commit -m "fix(drag-drop): address integration issues in internal drag-and-drop"
```
