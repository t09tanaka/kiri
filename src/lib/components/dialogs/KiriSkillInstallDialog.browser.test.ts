import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/svelte';
import KiriSkillInstallDialog from './KiriSkillInstallDialog.svelte';
import type { SkillStatus } from '@/lib/services/skillInstallService';

const installStatus: SkillStatus = {
  action: 'install',
  source_version: '0.2.0',
  installed_version: null,
  install_path: '/Users/user/.claude/skills/kiri-cli/SKILL.md',
};

const upgradeStatus: SkillStatus = {
  action: 'upgrade',
  source_version: '0.2.0',
  installed_version: '0.1.0',
  install_path: '/Users/user/.claude/skills/kiri-cli/SKILL.md',
};

describe('KiriSkillInstallDialog Component (Browser)', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders install title when action is install', () => {
    render(KiriSkillInstallDialog, {
      props: {
        status: installStatus,
        onAccept: vi.fn(),
        onDismiss: vi.fn(),
      },
    });

    expect(screen.getByText('Claude skill をインストール')).toBeInTheDocument();
  });

  it('renders upgrade title when action is upgrade', () => {
    render(KiriSkillInstallDialog, {
      props: {
        status: upgradeStatus,
        onAccept: vi.fn(),
        onDismiss: vi.fn(),
      },
    });

    expect(screen.getByText('Claude skill をアップデート')).toBeInTheDocument();
  });

  it('renders version numbers in upgrade mode', () => {
    const { container } = render(KiriSkillInstallDialog, {
      props: {
        status: upgradeStatus,
        onAccept: vi.fn(),
        onDismiss: vi.fn(),
      },
    });

    const text = container.textContent ?? '';
    expect(text).toContain('0.1.0');
    expect(text).toContain('0.2.0');
  });

  it('calls onAccept when primary button is clicked', async () => {
    const onAccept = vi.fn().mockResolvedValue(undefined);
    const onDismiss = vi.fn();

    render(KiriSkillInstallDialog, {
      props: { status: installStatus, onAccept, onDismiss },
    });

    const primaryBtn = screen.getByRole('button', { name: /インストール/i });
    await fireEvent.click(primaryBtn);

    expect(onAccept).toHaveBeenCalledOnce();
  });

  it('calls onDismiss when ghost button is clicked', async () => {
    const onAccept = vi.fn();
    const onDismiss = vi.fn();

    render(KiriSkillInstallDialog, {
      props: { status: installStatus, onAccept, onDismiss },
    });

    const dismissBtn = screen.getByRole('button', { name: /あとで/i });
    await fireEvent.click(dismissBtn);

    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it('calls onDismiss when Escape key is pressed', async () => {
    const onAccept = vi.fn();
    const onDismiss = vi.fn();

    render(KiriSkillInstallDialog, {
      props: { status: installStatus, onAccept, onDismiss },
    });

    await fireEvent.keyDown(document, { key: 'Escape' });

    expect(onDismiss).toHaveBeenCalledOnce();
  });
});
