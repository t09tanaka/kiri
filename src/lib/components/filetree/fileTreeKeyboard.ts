import type { FileTreeAction } from './fileTreeRegistry';

/**
 * Whether the given event target lives inside the file tree (or is the
 * document body). Restricts global hotkeys like F2/Delete/Cmd+N to the
 * file tree so they don't fight with the editor pane or terminal.
 */
export function isFileTreeFocusContext(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return target === null;
  if (target.matches('input, textarea, [contenteditable="true"]')) return false;
  if (target === document.body) return true;
  return target.closest('.file-tree') !== null;
}

/**
 * Identify which file tree action a keydown event maps to, or `null`
 * for keys that the tree does not own. Returned 'undo' is handled by
 * the caller (it does not target a specific path).
 *
 * Caller decides whether to fire the action against the selected entry
 * or fall back to a root-level action.
 */
export type KeyboardIntent = { kind: 'undo' } | { kind: 'action'; action: FileTreeAction } | null;

export function resolveKeyboardIntent(event: KeyboardEvent): KeyboardIntent {
  const mod = event.metaKey || event.ctrlKey;
  const key = event.key.toLowerCase();

  if (mod && !event.shiftKey && key === 'z') {
    return { kind: 'undo' };
  }
  if (mod && event.shiftKey && key === 'n') {
    return { kind: 'action', action: 'new-folder' };
  }
  if (mod && !event.shiftKey && key === 'n') {
    return { kind: 'action', action: 'new-file' };
  }
  if (event.key === 'F2') {
    return { kind: 'action', action: 'rename' };
  }
  if (event.key === 'Delete' || event.key === 'Backspace') {
    return { kind: 'action', action: 'delete' };
  }
  return null;
}

/**
 * Generate a unique "untitled-N" / "untitled-N.txt" name that does not
 * collide with the given existing names.
 */
export function generateUntitledName(
  action: 'new-file' | 'new-folder',
  existing: Set<string>
): string {
  const baseName = action === 'new-file' ? 'untitled.txt' : 'untitled-folder';
  if (!existing.has(baseName)) return baseName;

  let name = baseName;
  let counter = 1;
  while (existing.has(name)) {
    counter++;
    if (action === 'new-file') {
      const dot = baseName.lastIndexOf('.');
      name =
        dot > 0
          ? `${baseName.slice(0, dot)}-${counter}${baseName.slice(dot)}`
          : `${baseName}-${counter}`;
    } else {
      name = `${baseName}-${counter}`;
    }
  }
  return name;
}
