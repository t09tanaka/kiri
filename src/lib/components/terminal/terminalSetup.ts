import type { Terminal as TerminalType } from '@xterm/xterm';
import { peekStore } from '@/lib/stores/peekStore';
import { openerService } from '@/lib/services/openerService';
import { getTerminalSequence, isMacOS } from '@/lib/utils/terminalKeys';
import { mistTheme } from './terminalTheme';

export const TERMINAL_SCROLLBACK_LINES = 3000;

/**
 * Build the xterm options object we use everywhere a terminal is
 * constructed. Keeping this here means there's a single source of truth
 * for theme, scrollback, link handling, etc.
 *
 * The OSC 8 link handler intercepts `file://` URLs and routes them
 * through the peek editor (extracting `:line:col` when present); other
 * URLs go through the system opener service.
 */
export function buildTerminalOptions(
  fontSize: number
): ConstructorParameters<typeof import('@xterm/xterm').Terminal>[0] {
  return {
    cursorBlink: true,
    cursorStyle: 'bar',
    cursorWidth: 2,
    fontFamily: "'JetBrains Mono', 'SF Mono', 'Fira Code', 'Menlo', monospace",
    fontSize,
    fontWeight: '400',
    fontWeightBold: '500',
    lineHeight: 1.2,
    letterSpacing: 0,
    allowTransparency: true,
    theme: mistTheme,
    scrollback: TERMINAL_SCROLLBACK_LINES,
    smoothScrollDuration: 150,
    macOptionIsMeta: true,
    altClickMovesCursor: true,
    // Match macOS Terminal behavior for ED2 (Erase in Display)
    // This prevents blank lines when CLI tools use screen clearing.
    scrollOnEraseInDisplay: true,
    linkHandler: {
      activate: (_event, uri) => {
        if (uri.startsWith('file://')) {
          const filePath = uri.replace('file://', '');
          // file:///path/to/file:42 or file:///path/to/file:42:7
          const match = filePath.match(/^(.+?):(\d+)(?::(\d+))?$/);
          if (match) {
            const [, path, line, column] = match;
            peekStore.open(path, parseInt(line, 10), column ? parseInt(column, 10) : undefined);
          } else {
            peekStore.open(filePath);
          }
        } else {
          openerService.openUrl(uri);
        }
      },
      allowNonHttpProtocols: true,
    },
  };
}

/**
 * Block xterm from processing keys we handle in our capture-phase
 * keyboard listener. Returning `false` from this handler prevents xterm
 * from dispatching those keys, so the capture-phase code can emit its
 * own PTY sequences.
 *
 * Shift+Enter → literal newline (VS Code parity).
 * Option/Cmd+Arrow on macOS → word/line navigation that xterm would
 * otherwise eat thanks to `macOptionIsMeta`.
 * Cmd+Backspace on macOS → kill-line, handled in capture phase.
 */
export function attachKeyEventFilter(terminal: TerminalType): void {
  terminal.attachCustomKeyEventHandler((event) => {
    if (event.type !== 'keydown') return true;

    if (event.key === 'Enter' && event.shiftKey) {
      return false;
    }

    if (isMacOS() && (event.key === 'ArrowLeft' || event.key === 'ArrowRight')) {
      if (event.altKey || event.metaKey) {
        return false;
      }
    }

    if (isMacOS() && event.key === 'Backspace' && event.metaKey) {
      return false;
    }

    return true;
  });
}

/**
 * Attach the capture-phase keyboard listener on `terminal.textarea` so
 * we can convert Shift+Enter and macOS Option/Cmd+Arrow into raw PTY
 * sequences before xterm sees them.
 */
export function attachCapturePhaseKeyboard(
  terminal: TerminalType,
  writeToPty: (data: string) => void
): void {
  terminal.textarea?.addEventListener(
    'keydown',
    (event) => {
      if (event.key === 'Enter' && event.shiftKey) {
        event.preventDefault();
        event.stopPropagation();
        writeToPty('\n');
        return;
      }

      if (isMacOS()) {
        const sequence = getTerminalSequence(event);
        if (sequence) {
          event.preventDefault();
          event.stopPropagation();
          writeToPty(sequence);
        }
      }
    },
    { capture: true }
  );
}

/**
 * Lazy-load the visual addons that aren't required for the first paint
 * (WebLinks for URL detection, Canvas for the renderer). Loading them
 * after `terminal.open()` keeps them off the critical path of the very
 * first pane render. Canvas is preferred over WebGL so we don't burn a
 * WebGL context per pane.
 */
export async function loadDeferredAddons(terminal: TerminalType): Promise<void> {
  const [{ WebLinksAddon }, { CanvasAddon }] = await Promise.all([
    import('@xterm/addon-web-links'),
    import('@xterm/addon-canvas'),
  ]);
  terminal.loadAddon(
    new WebLinksAddon((_event, uri) => {
      openerService.openUrl(uri);
    })
  );
  terminal.loadAddon(new CanvasAddon());
}
