# Drag & Drop to Subdirectories Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable dragging files from the OS and dropping them onto any directory in the file tree (not just the project root).

**Architecture:** Use Tauri's `tauri://drag-over` event (fires continuously during drag) with `document.elementFromPoint()` to detect which tree item the cursor is over. Each `FileTreeItem` gets `data-drop-path` and `data-drop-is-dir` attributes for target detection. Files dropped over a file item copy to its parent directory.

**Tech Stack:** Svelte 5, TypeScript, Tauri 2.x event API

---

### Task 1: Add `getParentDirectory` utility function

**Files:**
- Create: `src/lib/utils/dragDrop.ts`
- Create: `src/lib/utils/dragDrop.test.ts`

**Step 1: Write the failing test**

```typescript
// src/lib/utils/dragDrop.test.ts
import { describe, it, expect } from 'vitest';
import { getParentDirectory } from './dragDrop';

describe('getParentDirectory', () => {
  it('should return parent directory for a file path', () => {
    expect(getParentDirectory('/project/src/lib/utils/fileIcons.ts')).toBe(
      '/project/src/lib/utils'
    );
  });

  it('should return parent for a nested directory', () => {
    expect(getParentDirectory('/project/src/lib')).toBe('/project/src');
  });

  it('should return root for a top-level item', () => {
    expect(getParentDirectory('/project/file.txt')).toBe('/project');
  });

  it('should return slash for root-level path', () => {
    expect(getParentDirectory('/file.txt')).toBe('/');
  });

  it('should handle trailing slashes', () => {
    expect(getParentDirectory('/project/src/')).toBe('/project');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/utils/dragDrop.test.ts`
Expected: FAIL with "Cannot find module"

**Step 3: Write minimal implementation**

```typescript
// src/lib/utils/dragDrop.ts

/**
 * Get the parent directory path from a file/directory path.
 */
export function getParentDirectory(filePath: string): string {
  // Remove trailing slash
  const normalized = filePath.endsWith('/') ? filePath.slice(0, -1) : filePath;
  const lastSlash = normalized.lastIndexOf('/');
  if (lastSlash <= 0) return '/';
  return normalized.slice(0, lastSlash);
}
```

**Step 4: Run test to verify it passes**

Run: `npm run test -- src/lib/utils/dragDrop.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/utils/dragDrop.ts src/lib/utils/dragDrop.test.ts
git commit -m "feat(drag-drop): add getParentDirectory utility function"
```

---

### Task 2: Add `resolveDropTarget` utility function

**Files:**
- Modify: `src/lib/utils/dragDrop.ts`
- Modify: `src/lib/utils/dragDrop.test.ts`

**Step 1: Write the failing test**

Append to `src/lib/utils/dragDrop.test.ts`:

```typescript
import { resolveDropTarget } from './dragDrop';

describe('resolveDropTarget', () => {
  it('should return directory path when hovering over a directory', () => {
    expect(resolveDropTarget('/project/src', true, '/project')).toBe('/project/src');
  });

  it('should return parent directory when hovering over a file', () => {
    expect(resolveDropTarget('/project/src/app.ts', false, '/project')).toBe('/project/src');
  });

  it('should return rootPath when hovering over a root-level file', () => {
    expect(resolveDropTarget('/project/README.md', false, '/project')).toBe('/project');
  });

  it('should return null when path is null (no element found)', () => {
    expect(resolveDropTarget(null, false, '/project')).toBe(null);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/utils/dragDrop.test.ts`
Expected: FAIL with "resolveDropTarget is not a function"

**Step 3: Write minimal implementation**

Add to `src/lib/utils/dragDrop.ts`:

```typescript
/**
 * Resolve the drop target directory from the hovered element's data attributes.
 * - If hovering over a directory, returns that directory path.
 * - If hovering over a file, returns its parent directory.
 * - If no element, returns null (caller falls back to rootPath).
 */
export function resolveDropTarget(
  path: string | null,
  isDir: boolean,
  rootPath: string
): string | null {
  if (path === null) return null;
  if (isDir) return path;
  const parent = getParentDirectory(path);
  // If parent would be above rootPath, clamp to rootPath
  return parent.startsWith(rootPath) ? parent : rootPath;
}
```

**Step 4: Run test to verify it passes**

Run: `npm run test -- src/lib/utils/dragDrop.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/utils/dragDrop.ts src/lib/utils/dragDrop.test.ts
git commit -m "feat(drag-drop): add resolveDropTarget utility function"
```

---

### Task 3: Add `data-drop-path` and `data-drop-is-dir` attributes to FileTreeItem

**Files:**
- Modify: `src/lib/components/filetree/FileTreeItem.svelte:279` (the `tree-item-container` div)

**Step 1: Add data attributes to the container**

In `FileTreeItem.svelte`, change the `tree-item-container` div (line 279):

```svelte
<!-- Before -->
<div
  class="tree-item-container"
  role="treeitem"
  aria-selected={isSelected}
  tabindex={isSelected ? 0 : -1}
  onmouseenter={handleDragMouseEnter}
  onmouseleave={handleDragMouseLeave}
>

<!-- After -->
<div
  class="tree-item-container"
  role="treeitem"
  aria-selected={isSelected}
  tabindex={isSelected ? 0 : -1}
  onmouseenter={handleDragMouseEnter}
  onmouseleave={handleDragMouseLeave}
  data-drop-path={entry.path}
  data-drop-is-dir={entry.is_dir}
>
```

**Step 2: Run lint and type check**

Run: `npm run check && npm run lint`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib/components/filetree/FileTreeItem.svelte
git commit -m "feat(drag-drop): add data-drop-path/data-drop-is-dir attributes to FileTreeItem"
```

---

### Task 4: Add `tauri://drag-over` listener with `elementFromPoint` target detection

**Files:**
- Modify: `src/lib/components/filetree/FileTree.svelte`

**Step 1: Add the drag-over listener**

In `FileTree.svelte`, add at the top of `<script>`:

```typescript
import { resolveDropTarget } from '@/lib/utils/dragDrop';
```

Add a new variable for the unlisten function and throttle timer:

```typescript
let unlistenDragOver: UnlistenFn | null = null;
let dragOverThrottleTimer: ReturnType<typeof setTimeout> | null = null;
let lastDragOverTarget: string | null = null;
```

Add a `handleDragOver` function:

```typescript
function handleDragOver(position: { x: number; y: number }) {
  // Throttle to ~60fps
  if (dragOverThrottleTimer) return;
  dragOverThrottleTimer = setTimeout(() => {
    dragOverThrottleTimer = null;
  }, 16);

  const element = document.elementFromPoint(position.x, position.y);
  if (!element) {
    dragDropStore.setDropTarget(null);
    handleAutoExpandOnTargetChange(null);
    return;
  }

  const treeItem = element.closest('[data-drop-path]') as HTMLElement | null;
  if (!treeItem) {
    dragDropStore.setDropTarget(null);
    handleAutoExpandOnTargetChange(null);
    return;
  }

  const path = treeItem.dataset.dropPath ?? null;
  const isDir = treeItem.dataset.dropIsDir === 'true';
  const targetDir = resolveDropTarget(path, isDir, rootPath);

  dragDropStore.setDropTarget(targetDir);
  handleAutoExpandOnTargetChange(targetDir);
}
```

Add auto-expand logic (replaces the logic previously in `FileTreeItem`'s `handleDragMouseEnter`):

```typescript
function handleAutoExpandOnTargetChange(targetDir: string | null) {
  if (targetDir === lastDragOverTarget) return;

  // Clear timer for previous target
  if (lastDragOverTarget) {
    dragDropStore.clearHoverTimer(lastDragOverTarget);
  }
  lastDragOverTarget = targetDir;

  // Start timer for new target (if it's a directory)
  if (targetDir && targetDir !== rootPath) {
    dragDropStore.startHoverTimer(targetDir, () => {
      // Auto-expand will be triggered by the FileTreeItem watching for this
      // We emit a custom event that FileTreeItem can listen to
      window.dispatchEvent(new CustomEvent('drag-auto-expand', { detail: { path: targetDir } }));
    });
  }
}
```

Update `setupDragDropListeners()` to add `tauri://drag-over`:

```typescript
unlistenDragOver = await eventService.listenCurrentWindow<DragPayload>(
  'tauri://drag-over',
  (event) => {
    handleDragOver(event.payload.position);
  }
);
```

Update `cleanupDragDropListeners()`:

```typescript
if (unlistenDragOver) {
  unlistenDragOver();
  unlistenDragOver = null;
}
if (dragOverThrottleTimer) {
  clearTimeout(dragOverThrottleTimer);
  dragOverThrottleTimer = null;
}
lastDragOverTarget = null;
```

**Step 2: Run lint and type check**

Run: `npm run check && npm run lint`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib/components/filetree/FileTree.svelte
git commit -m "feat(drag-drop): add tauri://drag-over listener with elementFromPoint target detection"
```

---

### Task 5: Add auto-expand listener to FileTreeItem

**Files:**
- Modify: `src/lib/components/filetree/FileTreeItem.svelte`

**Step 1: Add auto-expand event listener**

In `FileTreeItem.svelte`, add `onMount`/`onDestroy` imports and auto-expand listener:

```typescript
import { onMount, onDestroy } from 'svelte';

let autoExpandHandler: ((e: Event) => void) | null = null;

onMount(() => {
  if (entry.is_dir) {
    autoExpandHandler = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail.path === entry.path && !expanded) {
        toggleExpand();
      }
    };
    window.addEventListener('drag-auto-expand', autoExpandHandler);
  }
});

onDestroy(() => {
  if (autoExpandHandler) {
    window.removeEventListener('drag-auto-expand', autoExpandHandler);
  }
});
```

**Step 2: Run lint and type check**

Run: `npm run check && npm run lint`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib/components/filetree/FileTreeItem.svelte
git commit -m "feat(drag-drop): add auto-expand listener on FileTreeItem for drag hover"
```

---

### Task 6: Add `.drop-target-child` style and parent-directory highlighting

**Files:**
- Modify: `src/lib/components/filetree/FileTreeItem.svelte`

**Step 1: Add drop-target-child derived state**

In `FileTreeItem.svelte`, add a new derived for the "child of drop target" state. This highlights the hovered file while the parent directory gets `.drop-target`:

```typescript
// True when this file's parent directory is the current drop target
const isDropTargetChild = $derived(
  $isDragging && !entry.is_dir && $dropTargetPath !== null && entry.path.startsWith($dropTargetPath + '/')
  && !entry.path.slice($dropTargetPath.length + 1).includes('/')
);
```

**Step 2: Apply the class to the button element**

```svelte
<button
  class="tree-item"
  class:selected={isSelected}
  class:gitignored={entry.is_gitignored}
  class:directory={entry.is_dir}
  class:drop-target={isDropTarget}
  class:drop-target-child={isDropTargetChild}
  class:pending={isPending}
  ...
>
```

**Step 3: Add CSS style**

```css
/* Drop target child (file whose parent is the drop target) */
.tree-item.drop-target-child {
  background: rgba(125, 211, 252, 0.04);
}

.tree-item.drop-target-child .name {
  color: var(--accent-color);
  opacity: 0.7;
}
```

**Step 4: Run lint and type check**

Run: `npm run check && npm run lint`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/components/filetree/FileTreeItem.svelte
git commit -m "feat(drag-drop): add drop-target-child highlighting for files"
```

---

### Task 7: Extend preview entries to subdirectories

**Files:**
- Modify: `src/lib/components/filetree/FileTreeItem.svelte`

**Step 1: Add preview entries logic to FileTreeItem**

In `FileTreeItem.svelte`, import `draggedPaths` and add preview logic for subdirectories:

```typescript
import { dragDropStore, isDragging, dropTargetPath, draggedPaths } from '@/lib/stores/dragDropStore';

// Preview entries for this directory during drag
const previewEntries = $derived.by(() => {
  if (!$isDragging || !entry.is_dir || $dropTargetPath !== entry.path) return [];
  if (!expanded) return []; // Only show preview in expanded directories

  return $draggedPaths.map((sourcePath) => {
    const name = sourcePath.split('/').pop() || sourcePath;
    return {
      name,
      path: `${entry.path}/${name}`,
      is_dir: false,
      is_hidden: name.startsWith('.'),
      is_gitignored: false,
      is_pending: true,
    } satisfies FileEntry;
  });
});

// Sort helper
function sortEntries(items: FileEntry[]): FileEntry[] {
  return [...items].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
  });
}

// Combined children with previews
const displayChildren = $derived.by(() => {
  if (previewEntries.length === 0) return children;
  const previewPaths = new Set(previewEntries.map((e) => e.path));
  const filtered = children.filter((e) => !previewPaths.has(e.path));
  return sortEntries([...filtered, ...previewEntries]);
});
```

**Step 2: Update template to use displayChildren**

Change the children rendering (line 397):

```svelte
<!-- Before -->
{#each children as child, index (child.path)}

<!-- After -->
{#each displayChildren as child, index (child.path)}
```

**Step 3: Run lint and type check**

Run: `npm run check && npm run lint`
Expected: PASS

**Step 4: Commit**

```bash
git add src/lib/components/filetree/FileTreeItem.svelte
git commit -m "feat(drag-drop): extend preview entries to subdirectories"
```

---

### Task 8: Run all tests and verify

**Step 1: Run all unit tests**

Run: `npm run test`
Expected: All PASS (including new `dragDrop.test.ts`)

**Step 2: Run all tests including browser**

Run: `npm run test:all`
Expected: All PASS

**Step 3: Run lint and type check**

Run: `npm run check && npm run lint`
Expected: PASS

**Step 4: Commit if any fixes needed**

---

### Task 9: Verify with running app

**Step 1: Start the app**

Run: `npm run tauri dev`

**Step 2: Manual verification checklist**

- [ ] Drag a file from Finder onto a directory in the file tree → copies to that directory
- [ ] Drag a file over a file in the tree → copies to that file's parent directory
- [ ] Drag a file over empty area / project header → copies to project root
- [ ] Hover over a collapsed directory for 2 seconds → auto-expands
- [ ] Directory highlight (glow effect) shows on the correct target directory
- [ ] File highlight (subtle) shows when hovering over a file
- [ ] Preview entries appear in the target directory during drag
- [ ] Multiple files drag works correctly
- [ ] Duplicate file naming (conflict resolution) still works

**Step 3: Take screenshot to confirm**

Use `@hypothesi/tauri-mcp-server` to take screenshot and verify visual feedback.
