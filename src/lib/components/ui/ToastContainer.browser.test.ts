import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';

// Simple mock store state - managed outside vi.hoisted
let currentToasts: Array<{
  id: string;
  message: string;
  type: 'info' | 'success' | 'warning' | 'error';
  duration: number;
}> = [];
const subscribers: Set<(value: typeof currentToasts) => void> = new Set();

// Mock toastStore before importing the component
vi.mock('@/lib/stores/toastStore', () => ({
  toastStore: {
    subscribe: (fn: (value: typeof currentToasts) => void) => {
      subscribers.add(fn);
      fn(currentToasts);
      return () => subscribers.delete(fn);
    },
    remove: vi.fn(),
  },
}));

// Import after mock is set up
import ToastContainer from './ToastContainer.svelte';

// Helper to update mock store (exported for potential future use)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
function setMockToasts(
  toasts: Array<{
    id: string;
    message: string;
    type: 'info' | 'success' | 'warning' | 'error';
    duration: number;
  }>
) {
  currentToasts = toasts;
  subscribers.forEach((fn) => fn(currentToasts));
}

describe('ToastContainer Component (Browser)', () => {
  beforeEach(() => {
    currentToasts = [];
    subscribers.clear();
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders toast container', () => {
    const { container } = render(ToastContainer);

    expect(container.querySelector('.toast-container')).toBeInTheDocument();
  });

  it('renders no toasts when store is empty', () => {
    const { container } = render(ToastContainer);

    const toasts = container.querySelectorAll('[role="alert"]');
    expect(toasts).toHaveLength(0);
  });

  it('container element exists', () => {
    const { container } = render(ToastContainer);

    const toastContainer = container.querySelector('.toast-container');
    expect(toastContainer).toBeInTheDocument();
  });
});
