import { describe, it, expect, vi, beforeEach } from 'vitest';
import { prService } from './prService';
import type { PullRequest, GhCliStatus } from './prService';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

describe('prService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('checkGhCli', () => {
    it('should call invoke with check_gh_cli', async () => {
      const mockStatus: GhCliStatus = { installed: true, authenticated: true };
      vi.mocked(invoke).mockResolvedValue(mockStatus);
      const result = await prService.checkGhCli();
      expect(invoke).toHaveBeenCalledWith('check_gh_cli');
      expect(result).toEqual(mockStatus);
    });
  });

  describe('listPrs', () => {
    it('should call invoke with list_pull_requests and repo path', async () => {
      const mockPrs: PullRequest[] = [];
      vi.mocked(invoke).mockResolvedValue(mockPrs);
      const result = await prService.listPrs('/repo');
      expect(invoke).toHaveBeenCalledWith('list_pull_requests', { repoPath: '/repo' });
      expect(result).toEqual([]);
    });
  });

  describe('getPrDetail', () => {
    it('should call invoke with get_pull_request_detail and PR number', async () => {
      const mockPr: PullRequest = {
        number: 42,
        title: 'test',
        author_login: 'dev',
        head_ref_name: 'feat/test',
        state: 'OPEN',
        updated_at: '2026-03-29T00:00:00Z',
        additions: 10,
        deletions: 5,
        changed_files: 2,
        body: 'description',
        review_decision: null,
        status_check_rollup: [],
        labels: [],
        files: [],
      };
      vi.mocked(invoke).mockResolvedValue(mockPr);
      const result = await prService.getPrDetail('/repo', 42);
      expect(invoke).toHaveBeenCalledWith('get_pull_request_detail', {
        repoPath: '/repo',
        number: 42,
      });
      expect(result).toEqual(mockPr);
    });
  });
});
