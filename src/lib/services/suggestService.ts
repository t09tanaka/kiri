import { invoke } from '@tauri-apps/api/core';

export interface Suggestion {
  text: string;
  kind: 'command' | 'history' | 'path';
}

// Cache for PATH commands (doesn't change often)
let pathCommandsCache: string[] | null = null;
let commandHistoryCache: string[] | null = null;

/**
 * Load PATH commands from the backend
 */
async function loadPathCommands(): Promise<string[]> {
  if (pathCommandsCache) {
    return pathCommandsCache;
  }
  try {
    pathCommandsCache = await invoke<string[]>('get_path_commands');
    return pathCommandsCache;
  } catch (error) {
    console.error('Failed to load PATH commands:', error);
    return [];
  }
}

/**
 * Load command history from the backend
 */
async function loadCommandHistory(): Promise<string[]> {
  if (commandHistoryCache) {
    return commandHistoryCache;
  }
  try {
    commandHistoryCache = await invoke<string[]>('get_command_history', { limit: 500 });
    return commandHistoryCache;
  } catch (error) {
    console.error('Failed to load command history:', error);
    return [];
  }
}

/**
 * Load file suggestions for path completion
 */
async function loadFileSuggestions(partialPath: string, cwd?: string): Promise<string[]> {
  try {
    return await invoke<string[]>('get_file_suggestions', {
      partialPath,
      cwd,
    });
  } catch (error) {
    console.error('Failed to load file suggestions:', error);
    return [];
  }
}

/**
 * Get suggestions based on current input
 */
export async function getSuggestions(
  input: string,
  cwd?: string,
  maxResults: number = 10
): Promise<Suggestion[]> {
  const trimmedInput = input.trim();

  if (!trimmedInput) {
    return [];
  }

  const suggestions: Suggestion[] = [];
  const seen = new Set<string>();

  // Check if input contains space (means we're past the command)
  const hasSpace = trimmedInput.includes(' ');
  const parts = trimmedInput.split(/\s+/);
  const lastPart = parts[parts.length - 1];

  if (
    hasSpace &&
    (lastPart.startsWith('./') || lastPart.startsWith('/') || lastPart.startsWith('~'))
  ) {
    // File path completion
    const files = await loadFileSuggestions(lastPart, cwd);
    for (const file of files) {
      if (suggestions.length >= maxResults) break;
      if (!seen.has(file)) {
        seen.add(file);
        suggestions.push({ text: file, kind: 'path' });
      }
    }
  } else if (!hasSpace) {
    // Command completion (first word)
    const query = trimmedInput.toLowerCase();

    // Search history first (most relevant)
    const history = await loadCommandHistory();
    for (const cmd of history) {
      if (suggestions.length >= maxResults) break;
      // Get first word of history command
      const firstWord = cmd.split(/\s+/)[0];
      if (firstWord.toLowerCase().startsWith(query) && !seen.has(firstWord)) {
        seen.add(firstWord);
        suggestions.push({ text: firstWord, kind: 'history' });
      }
    }

    // Then search PATH commands
    const commands = await loadPathCommands();
    for (const cmd of commands) {
      if (suggestions.length >= maxResults) break;
      if (cmd.toLowerCase().startsWith(query) && !seen.has(cmd)) {
        seen.add(cmd);
        suggestions.push({ text: cmd, kind: 'command' });
      }
    }
  }

  return suggestions;
}

/**
 * Clear the cache (call when terminal is closed or on demand)
 */
export function clearCache() {
  pathCommandsCache = null;
  commandHistoryCache = null;
}

/**
 * Preload suggestions in the background
 */
export async function preloadSuggestions() {
  await Promise.all([loadPathCommands(), loadCommandHistory()]);
}
