import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/svelte';
import ProgressBar from './ProgressBar.svelte';

describe('ProgressBar Component (Browser)', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders progress bar container', () => {
    const { container } = render(ProgressBar);

    const progressContainer = container.querySelector('.progress-container');
    expect(progressContainer).toBeInTheDocument();
  });

  it('renders progress track', () => {
    const { container } = render(ProgressBar);

    const track = container.querySelector('.progress-track');
    expect(track).toBeInTheDocument();
  });

  it('shows progress fill with correct width attribute', () => {
    const { container } = render(ProgressBar, { props: { value: 50 } });

    const fill = container.querySelector('.progress-fill');
    expect(fill).toBeInTheDocument();
    // Check the style attribute contains the percentage
    expect(fill?.getAttribute('style')).toContain('width: 50%');
  });

  it('clamps value to 0-100 range (overflow)', () => {
    const { container } = render(ProgressBar, { props: { value: 150 } });

    const fill = container.querySelector('.progress-fill');
    // Value should be clamped to 100%
    expect(fill?.getAttribute('style')).toContain('width: 100%');
  });

  it('clamps value to 0-100 range (underflow)', () => {
    const { container } = render(ProgressBar, { props: { value: -10 } });

    const fill = container.querySelector('.progress-fill');
    // Value should be clamped to 0%
    expect(fill?.getAttribute('style')).toContain('width: 0%');
  });

  it('shows label when showLabel is true', () => {
    render(ProgressBar, { props: { value: 75, showLabel: true } });

    const label = screen.getByText('75%');
    expect(label).toBeInTheDocument();
    expect(label).toHaveClass('progress-label');
  });

  it('does not show label by default', () => {
    const { container } = render(ProgressBar, { props: { value: 75 } });

    const label = container.querySelector('.progress-label');
    expect(label).not.toBeInTheDocument();
  });

  it('shows indeterminate animation when indeterminate is true', () => {
    const { container } = render(ProgressBar, { props: { indeterminate: true } });

    const indeterminateContainer = container.querySelector('.progress-indeterminate');
    expect(indeterminateContainer).toBeInTheDocument();

    const bar1 = container.querySelector('.bar-1');
    const bar2 = container.querySelector('.bar-2');
    expect(bar1).toBeInTheDocument();
    expect(bar2).toBeInTheDocument();
  });

  it('does not show label when indeterminate even if showLabel is true', () => {
    const { container } = render(ProgressBar, {
      props: { indeterminate: true, showLabel: true },
    });

    const label = container.querySelector('.progress-label');
    expect(label).not.toBeInTheDocument();
  });

  it('applies sm size class', () => {
    const { container } = render(ProgressBar, { props: { size: 'sm' } });

    const progressContainer = container.querySelector('.progress-container');
    expect(progressContainer).toHaveClass('sm');
  });

  it('applies md size class by default', () => {
    const { container } = render(ProgressBar);

    const progressContainer = container.querySelector('.progress-container');
    expect(progressContainer).toHaveClass('md');
  });

  it('rounds label percentage', () => {
    render(ProgressBar, { props: { value: 33.7, showLabel: true } });

    const label = screen.getByText('34%');
    expect(label).toBeInTheDocument();
  });
});
