import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import Skeleton from './Skeleton.svelte';

describe('Skeleton Component (Browser)', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders skeleton element', () => {
    const { container } = render(Skeleton);

    const skeleton = container.querySelector('.skeleton');
    expect(skeleton).toBeInTheDocument();
  });

  it('applies default height', () => {
    const { container } = render(Skeleton);

    const skeleton = container.querySelector('.skeleton');
    expect(skeleton).toHaveStyle('height: 16px');
  });

  it('applies custom width', () => {
    const { container } = render(Skeleton, { props: { width: '200px' } });

    const skeleton = container.querySelector('.skeleton');
    expect(skeleton).toHaveStyle('width: 200px');
  });

  it('applies custom height', () => {
    const { container } = render(Skeleton, { props: { height: '32px' } });

    const skeleton = container.querySelector('.skeleton');
    expect(skeleton).toHaveStyle('height: 32px');
  });

  it('applies custom borderRadius', () => {
    const { container } = render(Skeleton, { props: { borderRadius: '8px' } });

    const skeleton = container.querySelector('.skeleton');
    expect(skeleton).toHaveStyle('border-radius: 8px');
  });

  it('applies circular variant border-radius (50%)', () => {
    const { container } = render(Skeleton, { props: { variant: 'circular' } });

    const skeleton = container.querySelector('.skeleton');
    expect(skeleton).toHaveStyle('border-radius: 50%');
  });

  it('has shimmer element', () => {
    const { container } = render(Skeleton);

    const shimmer = container.querySelector('.shimmer');
    expect(shimmer).toBeInTheDocument();
  });

  it('prioritizes custom borderRadius over variant', () => {
    const { container } = render(Skeleton, {
      props: { variant: 'circular', borderRadius: '4px' },
    });

    const skeleton = container.querySelector('.skeleton');
    expect(skeleton).toHaveStyle('border-radius: 4px');
  });
});
