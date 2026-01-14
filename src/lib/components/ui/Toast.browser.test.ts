import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/svelte';
import Toast from './Toast.svelte';

describe('Toast Component (Browser)', () => {
  // Clean up DOM after each test
  afterEach(() => {
    cleanup();
  });

  it('renders with message', async () => {
    render(Toast, { props: { message: 'Test message' } });

    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText('Test message')).toBeInTheDocument();
  });

  it('applies correct variant class for info type', async () => {
    render(Toast, { props: { message: 'Info toast', type: 'info' } });

    const toast = screen.getByRole('alert');
    expect(toast).toHaveClass('info');
  });

  it('applies correct variant class for success type', async () => {
    render(Toast, { props: { message: 'Success toast', type: 'success' } });

    const toast = screen.getByRole('alert');
    expect(toast).toHaveClass('success');
  });

  it('applies correct variant class for warning type', async () => {
    render(Toast, { props: { message: 'Warning toast', type: 'warning' } });

    const toast = screen.getByRole('alert');
    expect(toast).toHaveClass('warning');
  });

  it('applies correct variant class for error type', async () => {
    render(Toast, { props: { message: 'Error toast', type: 'error' } });

    const toast = screen.getByRole('alert');
    expect(toast).toHaveClass('error');
  });

  it('has close button', async () => {
    render(Toast, { props: { message: 'Test message' } });

    const closeButton = screen.getByRole('button', { name: /close/i });
    expect(closeButton).toBeInTheDocument();
  });

  it('calls onClose when close button is clicked', async () => {
    const onClose = vi.fn();
    render(Toast, { props: { message: 'Test message', onClose, duration: 0 } });

    const closeButton = screen.getByRole('button', { name: /close/i });
    await fireEvent.click(closeButton);

    // Wait for the exit animation (300ms)
    await new Promise((resolve) => setTimeout(resolve, 350));

    expect(onClose).toHaveBeenCalled();
  });

  it('shows progress bar with correct duration', async () => {
    render(Toast, { props: { message: 'Test message', duration: 5000 } });

    const toast = screen.getByRole('alert');
    const progressBar = toast.querySelector('.toast-progress');

    expect(progressBar).toBeInTheDocument();
    // Browser normalizes 5000ms to 5s
    expect(progressBar).toHaveStyle('animation-duration: 5s');
  });

  it('becomes visible after mount', async () => {
    render(Toast, { props: { message: 'Test message' } });

    // Wait for requestAnimationFrame
    await new Promise((resolve) => setTimeout(resolve, 50));

    const toast = screen.getByRole('alert');
    expect(toast).toHaveClass('visible');
  });
});
