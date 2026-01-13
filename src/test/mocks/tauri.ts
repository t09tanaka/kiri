import { vi } from 'vitest';

// Mock for @tauri-apps/api/core
export const invoke = vi.fn();

// Mock for @tauri-apps/plugin-store
export const Store = {
  load: vi.fn().mockResolvedValue({
    get: vi.fn(),
    set: vi.fn(),
    save: vi.fn(),
    delete: vi.fn(),
    reload: vi.fn(),
  }),
};

// Reset all mocks
export function resetTauriMocks() {
  invoke.mockReset();
  Store.load.mockClear();
}
