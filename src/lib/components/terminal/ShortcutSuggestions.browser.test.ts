import { render, fireEvent, cleanup } from '@testing-library/svelte';
import { describe, it, expect, vi, afterEach } from 'vitest';
import ShortcutSuggestions from './ShortcutSuggestions.svelte';
import type { InputRecord } from '@/lib/services/persistenceService';

describe('ShortcutSuggestions', () => {
  afterEach(() => {
    cleanup();
  });

  const makeSuggestion = (text: string, count: number): InputRecord => ({
    text: text.toLowerCase(),
    rawText: text,
    count,
    lastUsed: Date.now(),
    firstSeen: Date.now() - 100000,
    dismissedAt: null,
  });

  it('should not render badge when there are no suggestions', () => {
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions: [], onAdd: vi.fn(), onDismiss: vi.fn() },
    });
    expect(container.querySelector('.suggestion-badge')).toBeNull();
  });

  it('should render badge with count when there are suggestions', () => {
    const suggestions = [makeSuggestion('deploy', 5), makeSuggestion('test', 3)];
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd: vi.fn(), onDismiss: vi.fn() },
    });
    const badge = container.querySelector('.suggestion-badge');
    expect(badge).toBeTruthy();
    expect(badge!.textContent).toContain('+2');
  });

  it('should toggle popover on badge click', async () => {
    const suggestions = [makeSuggestion('deploy', 5)];
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd: vi.fn(), onDismiss: vi.fn() },
    });

    const badge = container.querySelector('.suggestion-badge')!;
    expect(container.querySelector('.suggestion-popover')).toBeNull();

    await fireEvent.click(badge);
    expect(container.querySelector('.suggestion-popover')).toBeTruthy();

    await fireEvent.click(badge);
    expect(container.querySelector('.suggestion-popover')).toBeNull();
  });

  it('should display suggestion text and count in popover', async () => {
    const suggestions = [makeSuggestion('deploy', 5)];
    const { container, getByText } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd: vi.fn(), onDismiss: vi.fn() },
    });

    await fireEvent.click(container.querySelector('.suggestion-badge')!);
    expect(getByText('deploy')).toBeTruthy();
    expect(getByText('5')).toBeTruthy();
  });

  it('should call onAdd when add button is clicked', async () => {
    const onAdd = vi.fn();
    const suggestions = [makeSuggestion('deploy', 5)];
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd, onDismiss: vi.fn() },
    });

    await fireEvent.click(container.querySelector('.suggestion-badge')!);
    const addBtn = container.querySelector('.suggestion-add-btn')!;
    await fireEvent.click(addBtn);
    expect(onAdd).toHaveBeenCalledWith(suggestions[0]);
  });

  it('should call onDismiss when dismiss button is clicked', async () => {
    const onDismiss = vi.fn();
    const suggestions = [makeSuggestion('deploy', 5)];
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd: vi.fn(), onDismiss },
    });

    await fireEvent.click(container.querySelector('.suggestion-badge')!);
    const dismissBtn = container.querySelector('.suggestion-dismiss-btn')!;
    await fireEvent.click(dismissBtn);
    expect(onDismiss).toHaveBeenCalledWith(suggestions[0]);
  });

  it('should close popover on outside click', async () => {
    const suggestions = [makeSuggestion('deploy', 5)];
    const { container } = render(ShortcutSuggestions, {
      props: { suggestions, onAdd: vi.fn(), onDismiss: vi.fn() },
    });

    // Open popover
    await fireEvent.click(container.querySelector('.suggestion-badge')!);
    expect(container.querySelector('.suggestion-popover')).toBeTruthy();

    // Click outside (on document.body)
    await fireEvent.click(document.body);
    expect(container.querySelector('.suggestion-popover')).toBeNull();
  });
});
