import type { Terminal as TerminalType } from '@xterm/xterm';

/**
 * Synchronized Output Mode (DEC Private Mode 2026).
 *
 * xterm.js does not implement Mode 2026 natively, so we sit in front of
 * `terminal.write()` and:
 *   1. Buffer any content between `\x1b[?2026h` and `\x1b[?2026l` so the
 *      frame is flushed atomically rather than letting xterm render
 *      partial CSI sequences.
 *   2. Batch all writes through a single `requestAnimationFrame` so
 *      multiple chunks coalesce into one paint.
 *   3. Drop pending output during a resize that happens AFTER initial
 *      setup. Ink-style apps redraw on SIGWINCH, and replaying old-size
 *      output corrupts the new layout.
 *
 * Setup ordering: call `markInitialSetupComplete()` once the PTY's
 * initial size has settled. Until then, resize-time drops are
 * suppressed because the shell's first prompt is genuinely valid.
 *
 * Refs: https://github.com/xtermjs/xterm.js/issues/3375
 */
export interface SyncOutputHandler {
  process(data: string): void;
  setResizing(resizing: boolean): void;
  markInitialSetupComplete(): void;
}

const SYNC_START = '\x1b[?2026h';
const SYNC_END = '\x1b[?2026l';
const MAX_SYNC_BUFFER_CHARS = 512 * 1024;
const MAX_PENDING_WRITE_CHARS = 512 * 1024;

function appendCapped(current: string, next: string, maxChars: number): string {
  if (!next) return current;
  const combined = current + next;
  if (combined.length <= maxChars) return combined;
  return combined.slice(combined.length - maxChars);
}

export function createSyncOutputHandler(
  terminal: TerminalType,
  options: { debug?: boolean } = {}
): SyncOutputHandler {
  let syncBuffer = '';
  let inSyncMode = false;
  let pendingWrite = '';
  let writeScheduled = false;
  let isResizing = false;
  let initialSetupComplete = false;
  let syncFrameCount = 0;
  const { debug = false } = options;

  const flushWrites = () => {
    if (pendingWrite) {
      if (isResizing && initialSetupComplete) {
        pendingWrite = '';
        writeScheduled = false;
        return;
      }
      terminal.write(pendingWrite);
      pendingWrite = '';
    }
    writeScheduled = false;
  };

  const scheduleWrite = (data: string) => {
    if (isResizing && initialSetupComplete) {
      return;
    }
    pendingWrite = appendCapped(pendingWrite, data, MAX_PENDING_WRITE_CHARS);
    if (!writeScheduled) {
      writeScheduled = true;
      requestAnimationFrame(flushWrites);
    }
  };

  return {
    process(data: string) {
      while (data.length > 0) {
        if (inSyncMode) {
          const endIndex = data.indexOf(SYNC_END);
          if (endIndex !== -1) {
            syncBuffer = appendCapped(
              syncBuffer,
              data.substring(0, endIndex),
              MAX_SYNC_BUFFER_CHARS
            );
            scheduleWrite(syncBuffer);
            syncFrameCount++;
            if (debug) {
              console.log(
                `[SyncOutput] Frame #${syncFrameCount} flushed (${syncBuffer.length} bytes)`
              );
            }
            syncBuffer = '';
            inSyncMode = false;
            data = data.substring(endIndex + SYNC_END.length);
          } else {
            syncBuffer = appendCapped(syncBuffer, data, MAX_SYNC_BUFFER_CHARS);
            data = '';
          }
        } else {
          const startIndex = data.indexOf(SYNC_START);
          if (startIndex !== -1) {
            if (startIndex > 0) {
              scheduleWrite(data.substring(0, startIndex));
            }
            inSyncMode = true;
            syncBuffer = '';
            if (debug) {
              console.log('[SyncOutput] Sync mode started');
            }
            data = data.substring(startIndex + SYNC_START.length);
          } else {
            scheduleWrite(data);
            data = '';
          }
        }
      }
    },
    setResizing(resizing) {
      isResizing = resizing;
    },
    markInitialSetupComplete() {
      initialSetupComplete = true;
    },
  };
}
