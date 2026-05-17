import { writable, get } from 'svelte/store';
import { fileService } from '@/lib/services/fileService';
import { toastStore } from '@/lib/stores/toastStore';

/// Max entries retained in the session undo stack. Bounded so a long
/// session of trash ops doesn't keep arbitrary references; older entries
/// silently drop off when the cap is hit.
const MAX_UNDO_ENTRIES = 20;

export interface UndoEntry {
  /// Currently only `trash` is undoable. `rename` / `create` etc. are
  /// reversible by the user via re-rename / re-delete and don't need a
  /// dedicated entry; keeping the stack to destructive ops only matches
  /// what #90 asks for ("undo for destructive file ops").
  type: 'trash';
  originalPath: string;
  /// Used for the toast confirmation after undo succeeds.
  displayName: string;
  trashedAt: number;
}

function createUndoStore() {
  const { subscribe, update, set } = writable<UndoEntry[]>([]);
  let supportedCache: boolean | null = null;

  async function ensureSupported(): Promise<boolean> {
    if (supportedCache === null) {
      try {
        supportedCache = await fileService.trashRestoreSupported();
      } catch {
        supportedCache = false;
      }
    }
    return supportedCache;
  }

  return {
    subscribe,

    /**
     * Record a successful trash op. Called by the delete pathway after
     * `move_to_trash` resolves so that Cmd+Z can revert it.
     */
    push(entry: UndoEntry) {
      update((stack) => {
        const next = [...stack, entry];
        if (next.length > MAX_UNDO_ENTRIES) {
          next.splice(0, next.length - MAX_UNDO_ENTRIES);
        }
        return next;
      });
    },

    /**
     * Pop the most recent entry and attempt to restore it. Returns
     * `true` if a restore was attempted (success or failure surfaced
     * via toast); `false` if there was nothing to undo or the platform
     * doesn't support restore (so callers can decide to ignore Cmd+Z).
     */
    async undo(): Promise<boolean> {
      const stack = get({ subscribe });
      if (stack.length === 0) return false;

      const supported = await ensureSupported();
      if (!supported) {
        toastStore.warning(
          "Undo isn't supported on this OS — open the system Trash to restore manually."
        );
        return false;
      }

      const entry = stack[stack.length - 1];
      // Optimistically pop so a quick Cmd+Z+Cmd+Z doesn't double-restore.
      update((s) => s.slice(0, -1));

      try {
        await fileService.restoreFromTrash(entry.originalPath);
        toastStore.success(`Restored ${entry.displayName}`);
        return true;
      } catch (e) {
        // Restore failed (e.g. trash entry already drained). Put the
        // entry back so the user could retry, then surface the error.
        update((s) => [...s, entry]);
        toastStore.error(`Failed to restore ${entry.displayName}: ${String(e)}`);
        return true;
      }
    },

    clear() {
      set([]);
    },

    /**
     * Test/utility: force the cached platform-support flag.
     */
    _setSupportedForTest(value: boolean | null) {
      supportedCache = value;
    },
  };
}

export const fileOpUndoStore = createUndoStore();
