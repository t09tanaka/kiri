import { describe, it, expect } from 'vitest';
import {
  STARTUP_COMMANDS,
  DEFAULT_STARTUP_COMMAND,
  getStartupCommandString,
  type StartupCommand,
} from './persistenceService';

describe('StartupCommand', () => {
  describe('STARTUP_COMMANDS', () => {
    it('should have three options: none, claude, codex', () => {
      expect(STARTUP_COMMANDS).toHaveLength(3);
      expect(STARTUP_COMMANDS.map((c) => c.id)).toEqual(['none', 'claude', 'codex']);
    });

    it('should have labels for each option', () => {
      expect(STARTUP_COMMANDS[0].label).toBe('None');
      expect(STARTUP_COMMANDS[1].label).toBe('Claude');
      expect(STARTUP_COMMANDS[2].label).toBe('Codex');
    });

    it('should have command strings (empty for none)', () => {
      expect(STARTUP_COMMANDS[0].command).toBe('');
      expect(STARTUP_COMMANDS[1].command).toBe('claude');
      expect(STARTUP_COMMANDS[2].command).toBe('codex');
    });
  });

  describe('DEFAULT_STARTUP_COMMAND', () => {
    it('should be none', () => {
      expect(DEFAULT_STARTUP_COMMAND).toBe('none');
    });
  });

  describe('getStartupCommandString', () => {
    it('should return empty string for none', () => {
      expect(getStartupCommandString('none')).toBe('');
    });

    it('should return claude for claude', () => {
      expect(getStartupCommandString('claude')).toBe('claude');
    });

    it('should return codex for codex', () => {
      expect(getStartupCommandString('codex')).toBe('codex');
    });

    it('should return empty string for unknown value', () => {
      expect(getStartupCommandString('unknown' as StartupCommand)).toBe('');
    });
  });
});
