import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/svelte';
import TabBar from './TabBar.svelte';
import type { Tab } from '@/lib/stores/tabStore';

// Mock tabStore
vi.mock('@/lib/stores/tabStore', () => ({
  tabStore: {
    setActiveTab: vi.fn(),
    closeTab: vi.fn(),
    addTerminalTab: vi.fn(),
  },
}));

// Mock fileIcons
vi.mock('@/lib/utils/fileIcons', () => ({
  getFileIconInfo: vi.fn().mockReturnValue({ icon: '📄', color: 'var(--text-primary)' }),
}));

describe('TabBar Component (Browser)', () => {
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

  it('shows modified indicator for modified files', () => {
    const { container } = render(TabBar, {
      props: { tabs: mockTabs, activeTabId: 'tab-1' },
    });

    const modifiedIndicators = container.querySelectorAll('.modified-indicator');
    expect(modifiedIndicators).toHaveLength(1);
  });

  it('has close button for each tab', () => {
    render(TabBar, { props: { tabs: mockTabs, activeTabId: 'tab-1' } });

    const closeButtons = screen.getAllByTitle('Close');
    expect(closeButtons).toHaveLength(3);
  });

  it('has add terminal button', () => {
    render(TabBar, { props: { tabs: mockTabs, activeTabId: 'tab-1' } });

    expect(screen.getByTitle('New Terminal (⌘`)')).toBeInTheDocument();
  });

  it('shows file path in title attribute for editor tabs', () => {
    render(TabBar, { props: { tabs: mockTabs, activeTabId: 'tab-1' } });

    const editorTab = screen.getByRole('tab', { name: /index\.ts/i });
    expect(editorTab).toHaveAttribute('title', '/project/src/index.ts');
  });

  it('shows terminal title in title attribute for terminal tabs', () => {
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
});
