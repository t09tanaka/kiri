import '@testing-library/jest-dom/vitest';
import { vi } from 'vitest';

// Mock Tauri services for browser testing
// Components should use services, which can be mocked

// Mock fileService
vi.mock('@/lib/services/fileService', () => ({
  fileService: {
    readFile: vi.fn().mockResolvedValue(''),
    writeFile: vi.fn().mockResolvedValue(undefined),
    readDirectory: vi.fn().mockResolvedValue([]),
    getHomeDirectory: vi.fn().mockResolvedValue('/home/user'),
    revealInFinder: vi.fn().mockResolvedValue(undefined),
  },
}));

// Mock terminalService
vi.mock('@/lib/services/terminalService', () => ({
  terminalService: {
    createTerminal: vi.fn().mockResolvedValue(1),
    writeTerminal: vi.fn().mockResolvedValue(undefined),
    resizeTerminal: vi.fn().mockResolvedValue(undefined),
    closeTerminal: vi.fn().mockResolvedValue(undefined),
  },
}));

// Mock eventService
vi.mock('@/lib/services/eventService', () => ({
  eventService: {
    listen: vi.fn().mockResolvedValue(() => {}),
    emit: vi.fn().mockResolvedValue(undefined),
  },
}));

// Mock dialogService
vi.mock('@/lib/services/dialogService', () => ({
  dialogService: {
    openDirectory: vi.fn().mockResolvedValue(null),
    openFile: vi.fn().mockResolvedValue(null),
  },
}));

// Mock windowService
vi.mock('@/lib/services/windowService', () => ({
  windowService: {
    createDiffViewWindow: vi.fn().mockResolvedValue(undefined),
  },
}));

// Mock watcherService
vi.mock('@/lib/services/watcherService', () => ({
  watcherService: {
    startWatching: vi.fn().mockResolvedValue(undefined),
    stopWatching: vi.fn().mockResolvedValue(undefined),
  },
}));
