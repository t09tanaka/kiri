import { writable, derived, get } from 'svelte/store';

interface DragDropState {
  isDragging: boolean;
  draggedPaths: string[];
  dropTargetPath: string | null;
}

const HOVER_EXPAND_DELAY = 2000; // 2 seconds

function createDragDropStore() {
  const { subscribe, set, update } = writable<DragDropState>({
    isDragging: false,
    draggedPaths: [],
    dropTargetPath: null,
  });

  const hoverTimers = new Map<string, ReturnType<typeof setTimeout>>();

  function clearAllTimers() {
    hoverTimers.forEach((timer) => clearTimeout(timer));
    hoverTimers.clear();
  }

  return {
    subscribe,

    /**
     * Start drag operation with the given paths
     */
    startDrag(paths: string[]) {
      set({
        isDragging: true,
        draggedPaths: paths,
        dropTargetPath: null,
      });
    },

    /**
     * End the current drag operation
     */
    endDrag() {
      clearAllTimers();
      set({
        isDragging: false,
        draggedPaths: [],
        dropTargetPath: null,
      });
    },

    /**
     * Set the current drop target path
     */
    setDropTarget(path: string | null) {
      update((state) => ({
        ...state,
        dropTargetPath: path,
      }));
    },

    /**
     * Start a hover timer for auto-expand functionality
     */
    startHoverTimer(path: string, onExpand: () => void) {
      // Clear existing timer for this path
      this.clearHoverTimer(path);

      const timer = setTimeout(() => {
        onExpand();
        hoverTimers.delete(path);
      }, HOVER_EXPAND_DELAY);

      hoverTimers.set(path, timer);
    },

    /**
     * Clear hover timer for a specific path
     */
    clearHoverTimer(path: string) {
      const timer = hoverTimers.get(path);
      if (timer) {
        clearTimeout(timer);
        hoverTimers.delete(path);
      }
    },

    /**
     * Clear all hover timers
     */
    clearAllHoverTimers() {
      clearAllTimers();
    },

    /**
     * Get current state snapshot
     */
    getState(): DragDropState {
      return get({ subscribe });
    },

    /**
     * Check if a specific path has an active hover timer
     */
    hasHoverTimer(path: string): boolean {
      return hoverTimers.has(path);
    },
  };
}

export const dragDropStore = createDragDropStore();

// Derived stores for convenience
export const isDragging = derived(dragDropStore, ($state) => $state.isDragging);
export const draggedPaths = derived(dragDropStore, ($state) => $state.draggedPaths);
export const dropTargetPath = derived(dragDropStore, ($state) => $state.dropTargetPath);
