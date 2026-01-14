import { listen, emit, type UnlistenFn, type EventCallback } from '@tauri-apps/api/event';

/**
 * Tauri event service
 * Wraps Tauri event API for testability
 */
export const eventService = {
  /**
   * Listen for a Tauri event
   */
  listen: <T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> =>
    listen(event, handler),

  /**
   * Emit a Tauri event
   */
  emit: <T>(event: string, payload: T): Promise<void> => emit(event, payload),
};

// Re-export types for convenience
export type { UnlistenFn, EventCallback };
