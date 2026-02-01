import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/svelte';
import TabBar from './TabBar.svelte';
import type { Tab, TerminalPane } from '@/lib/stores/tabStore';
import { tabStore } from '@/lib/stores/tabStore';
import { confirmDialogStore } from '@/lib/stores/confirmDialogStore';
import { terminalService } from '@/lib/services/terminalService';

// Mock tabStore
vi.mock('@/lib/stores/tabStore', () => ({
  tabStore: {
    setActiveTab: vi.fn(),
    closeTab: vi.fn(),
    addTerminalTab: vi.fn(),
  },
  getAllTerminalIds: vi.fn().mockReturnValue([1]),
}));

// Mock confirmDialogStore
vi.mock('@/lib/stores/confirmDialogStore', () => ({
  confirmDialogStore: {
    confirm: vi.fn(),
  },
}));

// Mock terminalService
vi.mock('@/lib/services/terminalService', () => ({
  terminalService: {
    isTerminalAlive: vi.fn(),
  },
}));

// Mock fileIcons
vi.mock('@/lib/utils/fileIcons', () => ({
  getFileIconInfo: vi.fn().mockReturnValue({ icon: '📄', color: 'var(--text-primary)' }),
}));

describe('TabBar Component (Browser)', () => {
  // Create a valid rootPane structure for terminal tabs
  const mockRootPane: TerminalPane = {
    id: 'pane-1',
    type: 'terminal',
    terminalId: 1,
    children: [],
    sizes: [],
    direction: 'horizontal',
  };

  const mockTabs: Tab[] = [
    {
      id: 'tab-1',
      type: 'editor',
      filePath: '/project/src/index.ts',
      title: 'index.ts',
      modified: false,
    },
    {
      id: 'tab-2',
      type: 'editor',
      filePath: '/project/src/App.svelte',
      title: 'App.svelte',
      modified: true,
    },
    {
      id: 'tab-3',
      type: 'terminal',
      filePath: '',
      title: 'Terminal 1',
      modified: false,
      rootPane: mockRootPane,
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders tab bar container', () => {
    const { container } = render(TabBar, {
      props: { tabs: mockTabs, activeTabId: 'tab-1' },
    });

    expect(container.querySelector('.tab-bar')).toBeInTheDocument();
  });

  it('renders all tabs', () => {
    render(TabBar, { props: { tabs: mockTabs, activeTabId: 'tab-1' } });

    expect(screen.getByText('index.ts')).toBeInTheDocument();
    expect(screen.getByText('App.svelte')).toBeInTheDocument();
    expect(screen.getByText('Terminal 1')).toBeInTheDocument();
  });

  it('renders tabs with correct role', () => {
    render(TabBar, { props: { tabs: mockTabs, activeTabId: 'tab-1' } });

    const tabs = screen.getAllByRole('tab');
    expect(tabs).toHaveLength(3);
  });

  it('highlights active tab', () => {
    const { container } = render(TabBar, {
      props: { tabs: mockTabs, activeTabId: 'tab-1' },
    });

    const tabs = container.querySelectorAll('.tab');
    expect(tabs[0]).toHaveClass('active');
    expect(tabs[1]).not.toHaveClass('active');
    expect(tabs[2]).not.toHaveClass('active');
  });

  it('has close button for each tab', () => {
    render(TabBar, { props: { tabs: mockTabs, activeTabId: 'tab-1' } });

    const closeButtons = screen.getAllByTitle('Close');
    expect(closeButtons).toHaveLength(3);
  });

  it('has add terminal button', () => {
    render(TabBar, { props: { tabs: mockTabs, activeTabId: 'tab-1' } });

    expect(screen.getByTitle('New Terminal (⌘T)')).toBeInTheDocument();
  });

  it('shows tab title in title attribute for editor tabs', () => {
    render(TabBar, { props: { tabs: mockTabs, activeTabId: 'tab-1' } });

    const editorTab = screen.getByRole('tab', { name: /index\.ts/i });
    expect(editorTab).toHaveAttribute('title', 'index.ts');
  });

  it('shows tab title in title attribute for terminal tabs', () => {
    render(TabBar, { props: { tabs: mockTabs, activeTabId: 'tab-1' } });

    const terminalTab = screen.getByRole('tab', { name: /terminal 1/i });
    expect(terminalTab).toHaveAttribute('title', 'Terminal 1');
  });

  it('renders active indicator for active tab', () => {
    const { container } = render(TabBar, {
      props: { tabs: mockTabs, activeTabId: 'tab-1' },
    });

    const activeIndicators = container.querySelectorAll('.active-indicator');
    expect(activeIndicators).toHaveLength(1);
  });

  it('renders no tabs when empty', () => {
    const { container } = render(TabBar, {
      props: { tabs: [], activeTabId: null },
    });

    const tabs = container.querySelectorAll('.tab');
    expect(tabs).toHaveLength(0);
  });

  it('has tab icons', () => {
    const { container } = render(TabBar, {
      props: { tabs: mockTabs, activeTabId: 'tab-1' },
    });

    const tabIcons = container.querySelectorAll('.tab-icon');
    expect(tabIcons).toHaveLength(3);
  });

  it('renders SVG icon for terminal tabs', () => {
    const { container } = render(TabBar, {
      props: { tabs: mockTabs, activeTabId: 'tab-3' },
    });

    // Get the terminal tab (third one)
    const tabIcons = container.querySelectorAll('.tab-icon');
    const terminalIcon = tabIcons[2];
    expect(terminalIcon.querySelector('svg')).toBeInTheDocument();
  });

  describe('Terminal close confirmation', () => {
    it('shows confirmation dialog when closing terminal tab with running process', async () => {
      // Terminal is alive (has running process)
      vi.mocked(terminalService.isTerminalAlive).mockResolvedValue(true);
      vi.mocked(confirmDialogStore.confirm).mockResolvedValue(true);

      const { container } = render(TabBar, {
        props: { tabs: mockTabs, activeTabId: 'tab-3' },
      });

      // Find the terminal tab's close button (third tab)
      const closeButtons = container.querySelectorAll('.close-btn');
      const terminalCloseBtn = closeButtons[2];

      await fireEvent.click(terminalCloseBtn);

      await waitFor(() => {
        expect(confirmDialogStore.confirm).toHaveBeenCalledWith({
          title: 'Close Terminal',
          message:
            'Are you sure you want to close this terminal? Any running processes will be terminated.',
          confirmLabel: 'Close',
          cancelLabel: 'Cancel',
          kind: 'warning',
        });
      });
    });

    it('closes terminal tab when user confirms', async () => {
      vi.mocked(terminalService.isTerminalAlive).mockResolvedValue(true);
      vi.mocked(confirmDialogStore.confirm).mockResolvedValue(true);

      const { container } = render(TabBar, {
        props: { tabs: mockTabs, activeTabId: 'tab-3' },
      });

      const closeButtons = container.querySelectorAll('.close-btn');
      const terminalCloseBtn = closeButtons[2];

      await fireEvent.click(terminalCloseBtn);

      await waitFor(() => {
        expect(tabStore.closeTab).toHaveBeenCalledWith('tab-3');
      });
    });

    it('does not close terminal tab when user cancels', async () => {
      vi.mocked(terminalService.isTerminalAlive).mockResolvedValue(true);
      vi.mocked(confirmDialogStore.confirm).mockResolvedValue(false);

      const { container } = render(TabBar, {
        props: { tabs: mockTabs, activeTabId: 'tab-3' },
      });

      const closeButtons = container.querySelectorAll('.close-btn');
      const terminalCloseBtn = closeButtons[2];

      await fireEvent.click(terminalCloseBtn);

      // Wait for the async operation to complete
      await waitFor(() => {
        expect(confirmDialogStore.confirm).toHaveBeenCalled();
      });

      expect(tabStore.closeTab).not.toHaveBeenCalled();
    });

    it('closes terminal without confirmation when no process is running', async () => {
      // Terminal is not alive (no running process)
      vi.mocked(terminalService.isTerminalAlive).mockResolvedValue(false);

      const { container } = render(TabBar, {
        props: { tabs: mockTabs, activeTabId: 'tab-3' },
      });

      const closeButtons = container.querySelectorAll('.close-btn');
      const terminalCloseBtn = closeButtons[2];

      await fireEvent.click(terminalCloseBtn);

      await waitFor(() => {
        // Should close the tab
        expect(tabStore.closeTab).toHaveBeenCalledWith('tab-3');
      });

      // Should not show confirmation
      expect(confirmDialogStore.confirm).not.toHaveBeenCalled();
    });

    it('does not show confirmation dialog when closing editor tab', async () => {
      const { container } = render(TabBar, {
        props: { tabs: mockTabs, activeTabId: 'tab-1' },
      });

      // Find the first editor tab's close button
      const closeButtons = container.querySelectorAll('.close-btn');
      const editorCloseBtn = closeButtons[0];

      await fireEvent.click(editorCloseBtn);

      await waitFor(() => {
        expect(tabStore.closeTab).toHaveBeenCalledWith('tab-1');
      });

      expect(confirmDialogStore.confirm).not.toHaveBeenCalled();
    });
  });
});
