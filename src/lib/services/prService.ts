import { invoke } from '@tauri-apps/api/core';

export interface GhCliStatus {
  installed: boolean;
  authenticated: boolean;
}

export interface CheckStatus {
  name: string;
  status: string;
  conclusion: string | null;
}

export interface PrLabel {
  name: string;
  color: string;
}

export interface PrFile {
  path: string;
  additions: number;
  deletions: number;
}

export interface PullRequest {
  number: number;
  title: string;
  author_login: string;
  head_ref_name: string;
  state: string;
  updated_at: string;
  additions: number;
  deletions: number;
  changed_files: number;
  body: string;
  review_decision: string | null;
  status_check_rollup: CheckStatus[];
  labels: PrLabel[];
  files: PrFile[];
}

export const prService = {
  checkGhCli: (): Promise<GhCliStatus> => invoke('check_gh_cli'),
  listPrs: (repoPath: string): Promise<PullRequest[]> => invoke('list_pull_requests', { repoPath }),
  getPrDetail: (repoPath: string, number: number): Promise<PullRequest> =>
    invoke('get_pull_request_detail', { repoPath, number }),
};
