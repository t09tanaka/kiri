import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/svelte';
import ContextMenu from './ContextMenu.svelte';
import type { MenuItem } from './ContextMenu.svelte';

describe('ContextMenu Component (Browser)', () => {
  const mockItems: MenuItem[] = [
    { id: 'copy', label: 'Copy', icon: '📋', shortcut: '⌘C' },
    { id: 'paste', label: 'Paste', icon: '📄', shortcut: '⌘V' },
    { id: 'sep1', label: '', separator: true },
    { id: 'delete', label: 'Delete', danger: true },
    { id: 'disabled', label: 'Disabled Item', disabled: true },
  ];

  afterEach(() => {
    cleanup();
  });

  it('renders context menu', () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    const menu = screen.getByRole('menu');
    expect(menu).toBeInTheDocument();
  });

  it('renders menu items', () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    expect(screen.getByText('Copy')).toBeInTheDocument();
    expect(screen.getByText('Paste')).toBeInTheDocument();
    expect(screen.getByText('Delete')).toBeInTheDocument();
  });

  it('renders menu item icons', () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    expect(screen.getByText('📋')).toBeInTheDocument();
    expect(screen.getByText('📄')).toBeInTheDocument();
  });

  it('renders shortcuts', () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    expect(screen.getByText('⌘C')).toBeInTheDocument();
    expect(screen.getByText('⌘V')).toBeInTheDocument();
  });

  it('renders separator', () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    const { container } = render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    const separator = container.querySelector('.separator');
    expect(separator).toBeInTheDocument();
  });

  it('calls onSelect when item is clicked', async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    const copyButton = screen.getByRole('menuitem', { name: /copy/i });
    await fireEvent.click(copyButton);

    expect(onSelect).toHaveBeenCalledWith('copy');
    expect(onClose).toHaveBeenCalled();
  });

  it('does not call onSelect for disabled items', async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    const disabledButton = screen.getByRole('menuitem', { name: /disabled item/i });
    await fireEvent.click(disabledButton);

    expect(onSelect).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it('applies danger class to danger items', () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    const deleteButton = screen.getByRole('menuitem', { name: /delete/i });
    expect(deleteButton).toHaveClass('danger');
  });

  it('applies disabled class to disabled items', () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    const disabledButton = screen.getByRole('menuitem', { name: /disabled item/i });
    expect(disabledButton).toHaveClass('disabled');
    expect(disabledButton).toBeDisabled();
  });

  it('calls onClose when Escape key is pressed', async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 100, y: 100, onSelect, onClose },
    });

    await fireEvent.keyDown(document, { key: 'Escape' });

    expect(onClose).toHaveBeenCalled();
  });

  it('positions menu at provided coordinates', () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(ContextMenu, {
      props: { items: mockItems, x: 150, y: 200, onSelect, onClose },
    });

    const menu = screen.getByRole('menu');
    // Initial position before adjustment
    expect(menu).toHaveStyle('left: 150px');
    expect(menu).toHaveStyle('top: 200px');
  });
});
