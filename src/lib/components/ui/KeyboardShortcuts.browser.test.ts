import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/svelte';
import KeyboardShortcuts from './KeyboardShortcuts.svelte';

describe('KeyboardShortcuts Component (Browser)', () => {
  afterEach(() => {
    cleanup();
  });

  it('does not render dialog when isOpen is false', () => {
    render(KeyboardShortcuts, { props: { isOpen: false, onClose: vi.fn() } });

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders dialog when isOpen is true', async () => {
    render(KeyboardShortcuts, { props: { isOpen: true, onClose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });
  });

  it('displays dialog title', async () => {
    render(KeyboardShortcuts, { props: { isOpen: true, onClose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();
    });
  });

  it('displays all shortcut groups', async () => {
    render(KeyboardShortcuts, { props: { isOpen: true, onClose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText('General')).toBeInTheDocument();
      expect(screen.getByText('Tabs')).toBeInTheDocument();
      expect(screen.getByText('View')).toBeInTheDocument();
      expect(screen.getByText('File Tree')).toBeInTheDocument();
    });
  });

  it('displays shortcut descriptions', async () => {
    render(KeyboardShortcuts, { props: { isOpen: true, onClose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText('Quick Open')).toBeInTheDocument();
      expect(screen.getByText('Toggle Sidebar')).toBeInTheDocument();
      expect(screen.getByText('New Terminal')).toBeInTheDocument();
    });
  });

  it('displays keyboard keys', async () => {
    const { container } = render(KeyboardShortcuts, {
      props: { isOpen: true, onClose: vi.fn() },
    });

    await waitFor(() => {
      const keys = container.querySelectorAll('kbd');
      expect(keys.length).toBeGreaterThan(0);
    });
  });

  it('has close button', async () => {
    render(KeyboardShortcuts, { props: { isOpen: true, onClose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /close/i })).toBeInTheDocument();
    });
  });

  it('calls onClose when close button is clicked', async () => {
    const onClose = vi.fn();
    render(KeyboardShortcuts, { props: { isOpen: true, onClose } });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /close/i })).toBeInTheDocument();
    });

    const closeButton = screen.getByRole('button', { name: /close/i });
    await fireEvent.click(closeButton);

    expect(onClose).toHaveBeenCalled();
  });

  it('calls onClose when Escape key is pressed', async () => {
    const onClose = vi.fn();
    render(KeyboardShortcuts, { props: { isOpen: true, onClose } });

    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    await fireEvent.keyDown(document, { key: 'Escape' });

    expect(onClose).toHaveBeenCalled();
  });

  it('displays brand footer', async () => {
    render(KeyboardShortcuts, { props: { isOpen: true, onClose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText('kiri')).toBeInTheDocument();
      expect(screen.getByText('霧')).toBeInTheDocument();
    });
  });

  it('displays escape hint in footer', async () => {
    render(KeyboardShortcuts, { props: { isOpen: true, onClose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText('Esc')).toBeInTheDocument();
      expect(screen.getByText('to close')).toBeInTheDocument();
    });
  });

  it('has backdrop element', async () => {
    const { container } = render(KeyboardShortcuts, {
      props: { isOpen: true, onClose: vi.fn() },
    });

    await waitFor(() => {
      const backdrop = container.querySelector('.backdrop');
      expect(backdrop).toBeInTheDocument();
    });
  });
});
