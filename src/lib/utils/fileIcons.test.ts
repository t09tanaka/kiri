import { describe, it, expect } from 'vitest';
import {
  getFileIconInfo,
  getFolderColor,
  isConfigFile,
  getTestFileBase,
  computeTestTreeLines,
} from './fileIcons';

describe('getFileIconInfo', () => {
  describe('special filenames', () => {
    it('should return npm icon for package.json', () => {
      const result = getFileIconInfo('package.json');
      expect(result.type).toBe('npm');
      expect(result.color).toBe('#cb3837');
    });

    it('should return git icon for .gitignore', () => {
      const result = getFileIconInfo('.gitignore');
      expect(result.type).toBe('git');
    });

    it('should return docker icon for Dockerfile', () => {
      const result = getFileIconInfo('Dockerfile');
      expect(result.type).toBe('docker');
      expect(result.color).toBe('#2496ed');
    });

    it('should return docker icon for Dockerfile variants', () => {
      expect(getFileIconInfo('Dockerfile.dev').type).toBe('docker');
      expect(getFileIconInfo('Dockerfile.prod').type).toBe('docker');
    });

    it('should return docker icon for compose files', () => {
      expect(getFileIconInfo('compose.yml').type).toBe('docker');
      expect(getFileIconInfo('compose.yaml').type).toBe('docker');
    });

    it('should return docker icon for .dockerignore', () => {
      expect(getFileIconInfo('.dockerignore').type).toBe('docker');
    });

    it('should handle case-insensitive filename matching via lowerFilename fallback', () => {
      // Test that PACKAGE.JSON (uppercase) matches package.json (lowercase in map)
      // This triggers the `filenameMap[lowerFilename]` branch in line 270
      const result = getFileIconInfo('PACKAGE.JSON');
      expect(result.type).toBe('npm');
      expect(result.color).toBe('#cb3837');
    });
  });

  describe('extension mapping', () => {
    it('should return typescript icon for .ts files', () => {
      const result = getFileIconInfo('index.ts');
      expect(result.type).toBe('typescript');
      expect(result.color).toBe('#64d2ff');
    });

    it('should return javascript icon for .js files', () => {
      const result = getFileIconInfo('script.js');
      expect(result.type).toBe('javascript');
      expect(result.color).toBe('#ffd60a');
    });

    it('should return svelte icon for .svelte files', () => {
      const result = getFileIconInfo('Component.svelte');
      expect(result.type).toBe('svelte');
      expect(result.color).toBe('#ff6b4a');
    });

    it('should return rust icon for .rs files', () => {
      const result = getFileIconInfo('main.rs');
      expect(result.type).toBe('rust');
      expect(result.color).toBe('#f29668');
    });

    it('should return python icon for .py files', () => {
      const result = getFileIconInfo('app.py');
      expect(result.type).toBe('python');
      expect(result.color).toBe('#3776ab');
    });

    it('should return dart icon for .dart files', () => {
      const result = getFileIconInfo('main.dart');
      expect(result.type).toBe('dart');
      expect(result.color).toBe('#00b4ab');
    });

    it('should return elixir icon for .ex files', () => {
      const result = getFileIconInfo('lib.ex');
      expect(result.type).toBe('elixir');
      expect(result.color).toBe('#6e4a7e');
    });

    it('should return terraform icon for .tf files', () => {
      const result = getFileIconInfo('main.tf');
      expect(result.type).toBe('terraform');
      expect(result.color).toBe('#7b42bc');
    });

    it('should return csv icon for .csv files', () => {
      const result = getFileIconInfo('data.csv');
      expect(result.type).toBe('csv');
      expect(result.color).toBe('#89d051');
    });

    it('should return handlebars icon for .hbs files', () => {
      const result = getFileIconInfo('template.hbs');
      expect(result.type).toBe('handlebars');
      expect(result.color).toBe('#f0772b');
    });
  });

  describe('compound extensions', () => {
    it('should return typescript-def icon for .d.ts files', () => {
      const result = getFileIconInfo('types.d.ts');
      expect(result.type).toBe('typescript-def');
    });

    it('should fall back to simple extension when compound extension is not registered', () => {
      // file.abc.ts should use .ts extension since .abc.ts is not registered
      const result = getFileIconInfo('file.abc.ts');
      expect(result.type).toBe('typescript');
    });
  });

  describe('config filename detection', () => {
    it('should return config icon for files containing "config"', () => {
      const result = getFileIconInfo('app.config.yaml');
      expect(result.type).toBe('config');
      expect(result.color).toBe('#6b7280');
    });

    it('should return config icon for vite.config.ts', () => {
      const result = getFileIconInfo('vite.config.ts');
      expect(result.type).toBe('config');
      expect(result.color).toBe('#6b7280');
    });

    it('should return config icon for tsconfig.json', () => {
      const result = getFileIconInfo('tsconfig.json');
      expect(result.type).toBe('config');
      expect(result.color).toBe('#6b7280');
    });

    it('should return config icon case-insensitively', () => {
      const result = getFileIconInfo('MyConfig.yml');
      expect(result.type).toBe('config');
    });

    it('should return config icon for .conf extension', () => {
      const result = getFileIconInfo('app.conf');
      expect(result.type).toBe('config');
      expect(result.color).toBe('#6b7280');
    });

    it('should return config icon for .cfg extension', () => {
      const result = getFileIconInfo('setup.cfg');
      expect(result.type).toBe('config');
      expect(result.color).toBe('#6b7280');
    });
  });

  describe('default behavior', () => {
    it('should return default file icon for unknown extensions', () => {
      const result = getFileIconInfo('unknown.xyz');
      expect(result.type).toBe('file');
      expect(result.color).toBe('#8b949e');
    });

    it('should return default file icon for files without extension', () => {
      const result = getFileIconInfo('noextension');
      expect(result.type).toBe('file');
    });
  });
});

describe('isConfigFile', () => {
  it('should return true for files containing "config"', () => {
    expect(isConfigFile('vite.config.ts')).toBe(true);
    expect(isConfigFile('tsconfig.json')).toBe(true);
    expect(isConfigFile('app.config.yaml')).toBe(true);
  });

  it('should return true case-insensitively', () => {
    expect(isConfigFile('MyConfig.yml')).toBe(true);
    expect(isConfigFile('CONFIG.js')).toBe(true);
  });

  it('should return true for .conf and .cfg extensions', () => {
    expect(isConfigFile('app.conf')).toBe(true);
    expect(isConfigFile('setup.cfg')).toBe(true);
  });

  it('should return false for non-config files', () => {
    expect(isConfigFile('index.ts')).toBe(false);
    expect(isConfigFile('README.md')).toBe(false);
    expect(isConfigFile('package.json')).toBe(false);
  });
});

describe('getTestFileBase', () => {
  it('should detect .test.ts files', () => {
    expect(getTestFileBase('dialogService.test.ts')).toBe('dialogService.ts');
  });

  it('should detect .spec.ts files', () => {
    expect(getTestFileBase('dialogService.spec.ts')).toBe('dialogService.ts');
  });

  it('should detect .browser.test.ts files', () => {
    expect(getTestFileBase('Button.browser.test.ts')).toBe('Button.ts');
  });

  it('should detect _test.go files', () => {
    expect(getTestFileBase('handler_test.go')).toBe('handler.go');
  });

  it('should return null for non-test files', () => {
    expect(getTestFileBase('dialogService.ts')).toBeNull();
    expect(getTestFileBase('README.md')).toBeNull();
    expect(getTestFileBase('test.ts')).toBeNull();
  });

  it('should handle config test files', () => {
    expect(getTestFileBase('eslint.config.test.js')).toBe('eslint.config.js');
  });
});

describe('computeTestTreeLines', () => {
  it('should mark single test file as last', () => {
    const items = [
      { name: 'foo.ts', path: '/foo.ts', is_dir: false },
      { name: 'foo.test.ts', path: '/foo.test.ts', is_dir: false },
      { name: 'bar.ts', path: '/bar.ts', is_dir: false },
    ];
    const result = computeTestTreeLines(items);
    expect(result.get('/foo.test.ts')).toBe('last');
    expect(result.has('/foo.ts')).toBe(false);
    expect(result.has('/bar.ts')).toBe(false);
  });

  it('should mark multiple test files with branch and last', () => {
    const items = [
      { name: 'foo.ts', path: '/foo.ts', is_dir: false },
      { name: 'foo.test.ts', path: '/foo.test.ts', is_dir: false },
      { name: 'foo.browser.test.ts', path: '/foo.browser.test.ts', is_dir: false },
      { name: 'bar.ts', path: '/bar.ts', is_dir: false },
    ];
    const result = computeTestTreeLines(items);
    expect(result.get('/foo.test.ts')).toBe('branch');
    expect(result.get('/foo.browser.test.ts')).toBe('last');
  });

  it('should skip directories', () => {
    const items = [
      { name: 'test', path: '/test', is_dir: true },
      { name: 'foo.test.ts', path: '/foo.test.ts', is_dir: false },
    ];
    const result = computeTestTreeLines(items);
    expect(result.get('/foo.test.ts')).toBe('last');
  });
});

describe('getFolderColor', () => {
  it('should return folder color when closed', () => {
    expect(getFolderColor(false)).toBe('#64d2ff');
  });

  it('should return folderOpen color when open', () => {
    expect(getFolderColor(true)).toBe('#70d7ff');
  });
});
