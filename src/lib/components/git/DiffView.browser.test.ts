import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, cleanup, waitFor } from '@testing-library/svelte';

// Types for mock data
interface GitStatusEntry {
  path: string;
  status: string;
}

interface GitFileDiff {
  path: string;
  status: string;
  diff: string;
}

interface GitRepoInfo {
  repoRoot: string;
  branch: string;
  statuses: GitStatusEntry[];
}

interface GitStoreState {
  repoInfo: GitRepoInfo | null;
  allDiffs: GitFileDiff[];
  isDiffsLoading: boolean;
  currentVisibleFile: string | null;
}

// Mock store state
let mockState: GitStoreState = {
  repoInfo: null,
  allDiffs: [],
  isDiffsLoading: false,
  currentVisibleFile: null,
};
const subscribers: Set<(value: GitStoreState) => void> = new Set();

// Mock gitStore before importing the component
vi.mock('@/lib/stores/gitStore', () => ({
  gitStore: {
    subscribe: (fn: (value: GitStoreState) => void) => {
      subscribers.add(fn);
      fn(mockState);
      return () => subscribers.delete(fn);
    },
    setCurrentVisibleFile: vi.fn(),
  },
  getStatusIcon: (status: string) => {
    const icons: Record<string, string> = {
      Modified: 'M',
      Added: 'A',
      Deleted: 'D',
      Untracked: 'U',
    };
    return icons[status] || '?';
  },
  getStatusColor: (status: string) => {
    const colors: Record<string, string> = {
      Modified: 'var(--git-modified)',
      Added: 'var(--git-added)',
      Deleted: 'var(--git-deleted)',
      Untracked: 'var(--git-untracked)',
    };
    return colors[status] || 'var(--text-muted)';
  },
}));

// Import after mock is set up
import DiffView from './DiffView.svelte';

// Helper to update mock store
function setMockState(newState: Partial<GitStoreState>) {
  mockState = { ...mockState, ...newState };
  subscribers.forEach((fn) => fn(mockState));
}

// Sample diff content for testing
const sampleDiff = `@@ -1,3 +1,4 @@
  line 1
  line 2
+ added line
  line 3`;

describe('DiffView Component (Browser)', () => {
  beforeEach(() => {
    mockState = {
      repoInfo: null,
      allDiffs: [],
      isDiffsLoading: false,
      currentVisibleFile: null,
    };
    subscribers.clear();
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders diff-view container', () => {
    const { container } = render(DiffView);

    expect(container.querySelector('.diff-view')).toBeInTheDocument();
  });

  it('shows "No changes" when allDiffs is empty', () => {
    const { container } = render(DiffView);

    expect(container.querySelector('.no-selection')).toBeInTheDocument();
    expect(container.textContent).toContain('No changes');
  });

  it('shows loading state when isDiffsLoading is true', () => {
    setMockState({ isDiffsLoading: true });

    const { container } = render(DiffView);

    expect(container.querySelector('.loading-state')).toBeInTheDocument();
    expect(container.textContent).toContain('Loading diffs');
  });

  it('renders file sections when diffs are available', async () => {
    setMockState({
      repoInfo: {
        repoRoot: '/test',
        branch: 'main',
        statuses: [{ path: 'test.ts', status: 'Modified' }],
      },
      allDiffs: [
        {
          path: 'test.ts',
          status: 'Modified',
          diff: sampleDiff,
        },
      ],
    });

    const { container } = render(DiffView);

    await waitFor(() => {
      expect(container.querySelector('.file-section')).toBeInTheDocument();
    });
  });

  it('displays file name in header', async () => {
    setMockState({
      repoInfo: {
        repoRoot: '/test',
        branch: 'main',
        statuses: [{ path: 'src/components/MyComponent.svelte', status: 'Modified' }],
      },
      allDiffs: [
        {
          path: 'src/components/MyComponent.svelte',
          status: 'Modified',
          diff: sampleDiff,
        },
      ],
    });

    const { container } = render(DiffView);

    await waitFor(() => {
      expect(container.textContent).toContain('MyComponent.svelte');
    });
  });

  it('displays change count badge when there are changes', async () => {
    setMockState({
      repoInfo: {
        repoRoot: '/test',
        branch: 'main',
        statuses: [
          { path: 'file1.ts', status: 'Modified' },
          { path: 'file2.ts', status: 'Added' },
        ],
      },
      allDiffs: [
        { path: 'file1.ts', status: 'Modified', diff: sampleDiff },
        { path: 'file2.ts', status: 'Added', diff: sampleDiff },
      ],
    });

    const { container } = render(DiffView);

    await waitFor(() => {
      const badge = container.querySelector('.badge');
      expect(badge).toBeInTheDocument();
      expect(badge?.textContent).toBe('2');
    });
  });

  it('does not throw state_unsafe_mutation error when rendering diffs', async () => {
    // Capture console errors
    const errors: string[] = [];
    const originalError = console.error;
    console.error = (...args) => {
      errors.push(args.map(String).join(' '));
      originalError.apply(console, args);
    };

    setMockState({
      repoInfo: {
        repoRoot: '/test',
        branch: 'main',
        statuses: [{ path: 'test.ts', status: 'Modified' }],
      },
      allDiffs: [
        {
          path: 'test.ts',
          status: 'Modified',
          diff: sampleDiff,
        },
      ],
    });

    render(DiffView);

    // Wait for rendering to complete
    await waitFor(() => {
      // Restore console.error
      console.error = originalError;

      // Check no state_unsafe_mutation errors
      const hasMutationError = errors.some((e) => e.includes('state_unsafe_mutation'));
      expect(hasMutationError).toBe(false);
    });
  });

  it('handles multiple files without errors', async () => {
    const errors: string[] = [];
    const originalError = console.error;
    console.error = (...args) => {
      errors.push(args.map(String).join(' '));
      originalError.apply(console, args);
    };

    // Create multiple files
    const files = Array.from({ length: 10 }, (_, i) => ({
      path: `file${i}.ts`,
      status: 'Modified',
      diff: sampleDiff,
    }));

    setMockState({
      repoInfo: {
        repoRoot: '/test',
        branch: 'main',
        statuses: files.map((f) => ({ path: f.path, status: f.status })),
      },
      allDiffs: files,
    });

    const { container } = render(DiffView);

    await waitFor(() => {
      console.error = originalError;

      // Should render all file sections
      const sections = container.querySelectorAll('.file-section');
      expect(sections.length).toBe(10);

      // No errors
      const hasMutationError = errors.some((e) => e.includes('state_unsafe_mutation'));
      expect(hasMutationError).toBe(false);
    });
  });

  it('displays status badge with correct styling', async () => {
    setMockState({
      repoInfo: {
        repoRoot: '/test',
        branch: 'main',
        statuses: [{ path: 'test.ts', status: 'Modified' }],
      },
      allDiffs: [
        {
          path: 'test.ts',
          status: 'Modified',
          diff: sampleDiff,
        },
      ],
    });

    const { container } = render(DiffView);

    await waitFor(() => {
      const statusBadge = container.querySelector('.status-badge');
      expect(statusBadge).toBeInTheDocument();
      expect(statusBadge?.textContent).toBe('M');
    });
  });
});
