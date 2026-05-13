import { render, fireEvent, cleanup } from '@testing-library/svelte';
import { describe, it, expect, vi, afterEach } from 'vitest';
import TerminalShortcutBar from './TerminalShortcutBar.svelte';

describe('TerminalShortcutBar', () => {
  afterEach(() => {
    cleanup();
  });

  const defaultProps = {
    visible: true,
    shortcuts: [
      { id: 'builtin-ok', label: 'OK', text: 'OK', builtin: true, type: 'reply' as const },
      {
        id: 'builtin-continue',
        label: 'Continue',
        text: 'continue',
        builtin: true,
        type: 'reply' as const,
      },
      { id: 'builtin-lgtm', label: 'LGTM', text: 'LGTM', builtin: true, type: 'reply' as const },
    ],
    showNumberRow: false,
    collapsed: false,
    onSend: vi.fn(),
    onSettingsClick: vi.fn(),
    onAddClick: vi.fn(),
    onToggleCollapse: vi.fn(),
  };

  it('should render shortcut buttons when visible', () => {
    const { getByText } = render(TerminalShortcutBar, { props: defaultProps });
    expect(getByText('OK')).toBeTruthy();
    expect(getByText('Continue')).toBeTruthy();
    expect(getByText('LGTM')).toBeTruthy();
  });

  it('should not render when not visible', () => {
    const { queryByText } = render(TerminalShortcutBar, {
      props: { ...defaultProps, visible: false },
    });
    expect(queryByText('OK')).toBeNull();
  });

  it('should call onSend with text and withEnter=true on click', async () => {
    const onSend = vi.fn();
    const { getByText } = render(TerminalShortcutBar, {
      props: { ...defaultProps, onSend },
    });
    await fireEvent.click(getByText('OK'));
    expect(onSend).toHaveBeenCalledWith('OK', true);
  });

  it('should call onSend with text and withEnter=false on shift+click', async () => {
    const onSend = vi.fn();
    const { getByText } = render(TerminalShortcutBar, {
      props: { ...defaultProps, onSend },
    });
    await fireEvent.click(getByText('OK'), { shiftKey: true });
    expect(onSend).toHaveBeenCalledWith('OK', false);
  });

  it('should render settings button', () => {
    const { getByTitle } = render(TerminalShortcutBar, { props: defaultProps });
    expect(getByTitle('Shortcut Settings')).toBeTruthy();
  });

  it('should call onSettingsClick when settings button is clicked', async () => {
    const onSettingsClick = vi.fn();
    const { getByTitle } = render(TerminalShortcutBar, {
      props: { ...defaultProps, onSettingsClick },
    });
    await fireEvent.click(getByTitle('Shortcut Settings'));
    expect(onSettingsClick).toHaveBeenCalled();
  });

  it('should render empty state when no shortcuts provided', () => {
    const { container } = render(TerminalShortcutBar, {
      props: { ...defaultProps, shortcuts: [] },
    });
    const buttons = container.querySelectorAll('.shortcut-btn');
    expect(buttons.length).toBe(0);
  });

  it('hides REPLY/CMD/PICK rows when collapsed=true', () => {
    const { queryByText } = render(TerminalShortcutBar, {
      props: { ...defaultProps, collapsed: true, onToggleCollapse: vi.fn() },
    });
    expect(queryByText('OK')).toBeNull();
    expect(queryByText('Continue')).toBeNull();
  });

  it('renders collapse button immediately before settings button', () => {
    const { container } = render(TerminalShortcutBar, {
      props: { ...defaultProps, collapsed: false, onToggleCollapse: vi.fn() },
    });
    const actions = container.querySelector('.bar-actions')!;
    const buttons = actions.querySelectorAll('button');
    expect(buttons.length).toBe(2);
    expect(buttons[0].classList.contains('collapse-btn')).toBe(true);
    expect(buttons[1].classList.contains('settings-btn')).toBe(true);
  });

  it('collapse button calls onToggleCollapse', async () => {
    const onToggleCollapse = vi.fn();
    const { container } = render(TerminalShortcutBar, {
      props: { ...defaultProps, collapsed: false, onToggleCollapse },
    });
    const btn = container.querySelector('.collapse-btn') as HTMLButtonElement;
    await fireEvent.click(btn);
    expect(onToggleCollapse).toHaveBeenCalledTimes(1);
  });

  it('collapse button title swaps on collapsed state', () => {
    const { container, rerender } = render(TerminalShortcutBar, {
      props: { ...defaultProps, collapsed: false, onToggleCollapse: vi.fn() },
    });
    let btn = container.querySelector('.collapse-btn') as HTMLButtonElement;
    expect(btn.title).toMatch(/minimize/i);

    rerender({ ...defaultProps, collapsed: true, onToggleCollapse: vi.fn() });
    btn = container.querySelector('.collapse-btn') as HTMLButtonElement;
    expect(btn.title).toMatch(/restore/i);
  });
});
