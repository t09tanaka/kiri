/**
 * Trap keyboard focus inside a container so Tab/Shift+Tab cycle through
 * the container's focusable descendants rather than escaping into the
 * surrounding app (e.g. the terminal underneath an open modal).
 *
 * Usage (Svelte):
 *
 *   onMount(() => {
 *     const release = trapFocus(containerEl);
 *     return release;
 *   });
 *
 * The trap re-queries focusable elements on every Tab press so it
 * stays correct as the modal's contents change.
 */

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'textarea:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

function getFocusable(container: HTMLElement): HTMLElement[] {
  const candidates = Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
  return candidates.filter((el) => {
    if (el.hasAttribute('disabled')) return false;
    if (el.getAttribute('aria-hidden') === 'true') return false;
    // offsetParent === null catches display:none / detached subtrees.
    // We deliberately don't check visibility:hidden because some hidden
    // patterns (e.g. visually-hidden focus targets) are intentional.
    if (el.offsetParent === null && getComputedStyle(el).position !== 'fixed') {
      return false;
    }
    return true;
  });
}

export interface FocusTrapOptions {
  /**
   * If true (default), focus the first focusable element when the trap
   * activates. Pass false when the caller has already programmatically
   * focused something inside the container.
   */
  autoFocus?: boolean;
}

/**
 * Attach a focus trap to `container`. Returns a cleanup function that
 * removes the listener. Safe to call multiple times for nested modals
 * — the most recently attached trap wins because keydown bubbles up
 * from the active modal.
 */
export function trapFocus(container: HTMLElement, options: FocusTrapOptions = {}): () => void {
  const { autoFocus = true } = options;

  if (autoFocus) {
    const focusable = getFocusable(container);
    if (focusable.length > 0 && !container.contains(document.activeElement)) {
      focusable[0].focus();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;

    const focusable = getFocusable(container);
    if (focusable.length === 0) {
      e.preventDefault();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    const active = document.activeElement as HTMLElement | null;

    // If focus has escaped the container entirely, pull it back in.
    if (!active || !container.contains(active)) {
      e.preventDefault();
      (e.shiftKey ? last : first).focus();
      return;
    }

    if (e.shiftKey && active === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && active === last) {
      e.preventDefault();
      first.focus();
    }
  }

  container.addEventListener('keydown', handleKeyDown);

  return () => {
    container.removeEventListener('keydown', handleKeyDown);
  };
}
