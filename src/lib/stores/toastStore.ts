import { writable } from 'svelte/store';

export interface Toast {
  id: string;
  message: string;
  type: 'info' | 'success' | 'warning' | 'error';
  duration: number;
}

function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);

  let idCounter = 0;

  return {
    subscribe,

    add(message: string, type: Toast['type'] = 'info', duration: number = 3000) {
      const id = `toast-${++idCounter}`;
      const toast: Toast = { id, message, type, duration };

      update((toasts) => [...toasts, toast]);

      return id;
    },

    remove(id: string) {
      update((toasts) => toasts.filter((t) => t.id !== id));
    },

    clear() {
      update(() => []);
    },

    // Convenience methods
    info(message: string, duration?: number) {
      return this.add(message, 'info', duration);
    },

    success(message: string, duration?: number) {
      return this.add(message, 'success', duration);
    },

    warning(message: string, duration?: number) {
      return this.add(message, 'warning', duration);
    },

    error(message: string, duration?: number) {
      return this.add(message, 'error', duration);
    },
  };
}

export const toastStore = createToastStore();
