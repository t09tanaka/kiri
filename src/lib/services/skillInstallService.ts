import { invoke } from '@tauri-apps/api/core';

export type InstallAction = 'install' | 'upgrade' | 'none';

export interface SkillStatus {
  action: InstallAction;
  source_version: string | null;
  installed_version: string | null;
  install_path: string;
}

export interface InstallReport {
  action: InstallAction;
  installed_version: string | null;
  install_path: string;
}

/**
 * Skill install operations service.
 * Wraps the kiri_skill_status / install_kiri_skill Tauri commands.
 *
 * The skill is the kiri-cli SKILL.md bundled with the app. We never auto-install:
 * the frontend checks status, shows a confirmation dialog when action != "none",
 * and only on user approval invokes install_kiri_skill (force: false).
 */
export const skillInstallService = {
  status: (): Promise<SkillStatus> => invoke('kiri_skill_status'),

  install: (force: boolean): Promise<InstallReport> => invoke('install_kiri_skill', { force }),
};
