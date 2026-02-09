import { listen, emit, type UnlistenFn, type EventCallback } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Tauri event service
 * Wraps Tauri event API for testability
 */
export const eventService = {
  /**
   * Listen for a Tauri event (global - receives from all windows)
   */
  listen: <T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> =>
    listen(event, handler),

  /**
   * Listen for a Tauri event scoped to the current window only
   */
  listenCurrentWindow: <T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> =>
    getCurrentWindow().listen(event, handler),

  /**
   * Emit a Tauri event
   */
  emit: <T>(event: string, payload: T): Promise<void> => emit(event, payload),
};

// Re-export types for convenience
export type { UnlistenFn, EventCallback };
