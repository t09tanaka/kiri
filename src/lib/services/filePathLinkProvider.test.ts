import { describe, it, expect, vi } from 'vitest';
import { parseFilePathsFromLine, createFilePathLinkProvider } from './filePathLinkProvider';
import type { Terminal, IBufferLine, IBuffer } from '@xterm/xterm';

describe('parseFilePathsFromLine', () => {
  describe('basic file paths', () => {
    it('should parse a simple file path', () => {
      const matches = parseFilePathsFromLine('src/file.ts');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('src/file.ts');
      expect(matches[0].line).toBeUndefined();
      expect(matches[0].column).toBeUndefined();
    });

    it('should parse file path with line number', () => {
      const matches = parseFilePathsFromLine('src/file.ts:42');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('src/file.ts');
      expect(matches[0].line).toBe(42);
      expect(matches[0].column).toBeUndefined();
    });

    it('should parse file path with line and column', () => {
      const matches = parseFilePathsFromLine('src/file.ts:42:10');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('src/file.ts');
      expect(matches[0].line).toBe(42);
      expect(matches[0].column).toBe(10);
    });
  });

  describe('relative paths', () => {
    it('should parse ./ relative path', () => {
      const matches = parseFilePathsFromLine('./components/App.svelte:10');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('./components/App.svelte');
      expect(matches[0].line).toBe(10);
    });

    it('should parse ../ relative path', () => {
      const matches = parseFilePathsFromLine('../utils/date.ts');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('../utils/date.ts');
    });
  });

  describe('absolute paths', () => {
    it('should parse absolute path', () => {
      const matches = parseFilePathsFromLine('/absolute/path/file.rs:100');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('/absolute/path/file.rs');
      expect(matches[0].line).toBe(100);
    });
  });

  describe('multiple file paths', () => {
    it('should parse multiple file paths in one line', () => {
      const matches = parseFilePathsFromLine(
        'Error in src/main.ts:10 imported from lib/utils.ts:20'
      );

      expect(matches).toHaveLength(2);
      expect(matches[0].path).toBe('src/main.ts');
      expect(matches[0].line).toBe(10);
      expect(matches[1].path).toBe('lib/utils.ts');
      expect(matches[1].line).toBe(20);
    });
  });

  describe('file paths with various extensions', () => {
    const extensions = [
      'ts',
      'tsx',
      'js',
      'jsx',
      'svelte',
      'rs',
      'md',
      'json',
      'toml',
      'css',
      'scss',
      'html',
      'yaml',
      'yml',
      'txt',
      'sh',
      'py',
      'go',
      'c',
      'cpp',
      'h',
      'hpp',
      'java',
      'rb',
      'php',
      'vue',
      'astro',
      'prisma',
    ];

    for (const ext of extensions) {
      it(`should parse file with .${ext} extension`, () => {
        const matches = parseFilePathsFromLine(`src/file.${ext}`);

        expect(matches).toHaveLength(1);
        expect(matches[0].path).toBe(`src/file.${ext}`);
      });
    }
  });

  describe('file paths in context', () => {
    it('should parse file path after whitespace', () => {
      const matches = parseFilePathsFromLine('  src/file.ts:10');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('src/file.ts');
    });

    it('should parse file path in quotes', () => {
      const matches = parseFilePathsFromLine('"src/file.ts:10"');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('src/file.ts');
    });

    it('should parse file path in single quotes', () => {
      const matches = parseFilePathsFromLine("'src/file.ts:10'");

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('src/file.ts');
    });

    it('should parse file path in parentheses', () => {
      const matches = parseFilePathsFromLine('(src/file.ts:10)');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('src/file.ts');
    });

    it('should parse file path in brackets', () => {
      const matches = parseFilePathsFromLine('[src/file.ts:10]');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('src/file.ts');
    });
  });

  describe('startIndex and endIndex', () => {
    it('should correctly calculate start and end indices', () => {
      const matches = parseFilePathsFromLine('Error: src/file.ts:10:5');

      expect(matches).toHaveLength(1);
      expect(matches[0].startIndex).toBe(7); // After "Error: "
      expect(matches[0].endIndex).toBe(23); // End of "src/file.ts:10:5"
    });

    it('should correctly calculate indices for multiple paths', () => {
      const line = 'from src/a.ts:1 to src/b.ts:2';
      const matches = parseFilePathsFromLine(line);

      expect(matches).toHaveLength(2);
      expect(line.substring(matches[0].startIndex, matches[0].endIndex)).toBe('src/a.ts:1');
      expect(line.substring(matches[1].startIndex, matches[1].endIndex)).toBe('src/b.ts:2');
    });
  });

  describe('edge cases', () => {
    it('should return empty array for empty string', () => {
      const matches = parseFilePathsFromLine('');
      expect(matches).toHaveLength(0);
    });

    it('should return empty array for string without file paths', () => {
      const matches = parseFilePathsFromLine('Hello world');
      expect(matches).toHaveLength(0);
    });

    it('should not match incomplete file paths', () => {
      const matches = parseFilePathsFromLine('file without extension');
      expect(matches).toHaveLength(0);
    });

    it('should handle file path at end of line', () => {
      const matches = parseFilePathsFromLine('Error in file.ts');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('file.ts');
    });

    it('should handle @ in path (node_modules)', () => {
      const matches = parseFilePathsFromLine('@types/node/index.ts');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('@types/node/index.ts');
    });

    it('should handle hyphen in file name', () => {
      const matches = parseFilePathsFromLine('my-file-name.ts');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('my-file-name.ts');
    });

    it('should handle dot in directory name', () => {
      const matches = parseFilePathsFromLine('.config/settings.json');

      expect(matches).toHaveLength(1);
      expect(matches[0].path).toBe('.config/settings.json');
    });
  });

  describe('regex state reset', () => {
    it('should correctly parse paths on consecutive calls', () => {
      const matches1 = parseFilePathsFromLine('src/file1.ts');
      const matches2 = parseFilePathsFromLine('src/file2.ts');

      expect(matches1).toHaveLength(1);
      expect(matches1[0].path).toBe('src/file1.ts');
      expect(matches2).toHaveLength(1);
      expect(matches2[0].path).toBe('src/file2.ts');
    });
  });
});

// Helper to create mock Terminal
function createMockTerminal(lines: (string | null)[]): Terminal {
  const mockBuffer: Partial<IBuffer> = {
    getLine: vi.fn((index: number): IBufferLine | undefined => {
      const lineText = lines[index];
      if (lineText === null || lineText === undefined) {
        return undefined;
      }
      return {
        translateToString: () => lineText,
      } as IBufferLine;
    }),
  };

  return {
    buffer: {
      active: mockBuffer as IBuffer,
    },
  } as Terminal;
}

describe('createFilePathLinkProvider', () => {
  describe('provideLinks', () => {
    it('should call callback with undefined when line does not exist', () => {
      const mockTerminal = createMockTerminal([null]);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      provider.provideLinks(1, callback);

      expect(callback).toHaveBeenCalledWith(undefined);
    });

    it('should call callback with undefined when no file paths found', () => {
      const mockTerminal = createMockTerminal(['Hello world with no paths']);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      provider.provideLinks(1, callback);

      expect(callback).toHaveBeenCalledWith(undefined);
    });

    it('should call callback with links when file paths found', () => {
      const mockTerminal = createMockTerminal(['Error in src/file.ts:10']);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      provider.provideLinks(1, callback);

      expect(callback).toHaveBeenCalledTimes(1);
      const links = callback.mock.calls[0][0];
      expect(links).toHaveLength(1);
      expect(links[0].text).toBe('src/file.ts:10');
      expect(links[0].range.start.y).toBe(1);
      expect(links[0].range.end.y).toBe(1);
    });

    it('should create links with correct range for file path with line and column', () => {
      const mockTerminal = createMockTerminal(['src/file.ts:42:10']);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      provider.provideLinks(1, callback);

      const links = callback.mock.calls[0][0];
      expect(links).toHaveLength(1);
      expect(links[0].text).toBe('src/file.ts:42:10');
      // x is 1-indexed in xterm
      expect(links[0].range.start.x).toBe(1);
      expect(links[0].range.end.x).toBe(18); // Length of "src/file.ts:42:10" + 1
    });

    it('should create multiple links for multiple file paths', () => {
      const mockTerminal = createMockTerminal(['from src/a.ts:1 to src/b.ts:2']);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      provider.provideLinks(1, callback);

      const links = callback.mock.calls[0][0];
      expect(links).toHaveLength(2);
      expect(links[0].text).toBe('src/a.ts:1');
      expect(links[1].text).toBe('src/b.ts:2');
    });

    it('should call onActivate with correct parameters when link is activated', () => {
      const mockTerminal = createMockTerminal(['src/file.ts:42:10']);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      provider.provideLinks(1, callback);

      const links = callback.mock.calls[0][0];
      const mockEvent = {} as MouseEvent;

      // Activate the link
      links[0].activate(mockEvent, links[0].text);

      expect(onActivate).toHaveBeenCalledWith('src/file.ts', 42, 10);
    });

    it('should call onActivate with undefined column when not specified', () => {
      const mockTerminal = createMockTerminal(['src/file.ts:42']);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      provider.provideLinks(1, callback);

      const links = callback.mock.calls[0][0];
      links[0].activate({} as MouseEvent, links[0].text);

      expect(onActivate).toHaveBeenCalledWith('src/file.ts', 42, undefined);
    });

    it('should call onActivate with undefined line and column when not specified', () => {
      const mockTerminal = createMockTerminal(['src/file.ts']);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      provider.provideLinks(1, callback);

      const links = callback.mock.calls[0][0];
      links[0].activate({} as MouseEvent, links[0].text);

      expect(onActivate).toHaveBeenCalledWith('src/file.ts', undefined, undefined);
    });

    it('should handle hover without error', () => {
      const mockTerminal = createMockTerminal(['src/file.ts']);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      provider.provideLinks(1, callback);

      const links = callback.mock.calls[0][0];

      // Hover should not throw
      expect(() => {
        links[0].hover?.({} as MouseEvent, links[0].text);
      }).not.toThrow();
    });

    it('should correctly handle bufferLineNumber (1-indexed)', () => {
      const mockTerminal = createMockTerminal(['line 0', 'src/file.ts in line 1', 'line 2']);
      const onActivate = vi.fn();
      const callback = vi.fn();

      const provider = createFilePathLinkProvider(mockTerminal, onActivate);
      // Request line 2 (which is index 1 in the buffer)
      provider.provideLinks(2, callback);

      const links = callback.mock.calls[0][0];
      expect(links).toHaveLength(1);
      expect(links[0].text).toBe('src/file.ts');
      expect(links[0].range.start.y).toBe(2);
      expect(links[0].range.end.y).toBe(2);
    });
  });
});
