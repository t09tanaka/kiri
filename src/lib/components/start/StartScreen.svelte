<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { projectStore, recentProjects, type RecentProject } from '@/lib/stores/projectStore';
  import RecentProjectItem from './RecentProjectItem.svelte';

  async function handleOpenDirectory() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Open Directory',
    });

    if (selected && typeof selected === 'string') {
      await projectStore.openProject(selected);
    }
  }

  function handleProjectSelect(project: RecentProject) {
    projectStore.openProject(project.path);
  }

  function handleProjectRemove(project: RecentProject) {
    projectStore.removeProject(project.path);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'o') {
      e.preventDefault();
      handleOpenDirectory();
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="start-screen">
  <div class="content">
    <header class="header">
      <h1 class="title">Kiri</h1>
      <p class="subtitle">Light as mist, powerful</p>
    </header>

    <button class="open-button" onclick={handleOpenDirectory}>
      <span class="open-icon">+</span>
      <span class="open-text">Open Directory...</span>
      <span class="open-shortcut">Cmd+O</span>
    </button>

    {#if $recentProjects.length > 0}
      <section class="recent-section">
        <h2 class="section-title">Recent Projects</h2>
        <div class="recent-list">
          {#each $recentProjects as project (project.path)}
            <RecentProjectItem
              {project}
              onSelect={() => handleProjectSelect(project)}
              onRemove={() => handleProjectRemove(project)}
            />
          {/each}
        </div>
      </section>
    {:else}
      <section class="empty-section">
        <p class="empty-text">No recent projects</p>
        <p class="empty-hint">Open a directory to get started</p>
      </section>
    {/if}
  </div>
</div>

<style>
  .start-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    background-color: var(--bg-primary);
  }

  .content {
    width: 100%;
    max-width: 500px;
    padding: 40px;
  }

  .header {
    text-align: center;
    margin-bottom: 40px;
  }

  .title {
    font-size: 48px;
    font-weight: 300;
    color: var(--text-primary);
    margin-bottom: 8px;
    letter-spacing: 2px;
  }

  .subtitle {
    font-size: 14px;
    color: var(--text-secondary);
    font-weight: 400;
  }

  .open-button {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 16px 20px;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 14px;
    cursor: pointer;
    transition:
      background-color 0.15s,
      border-color 0.15s;
  }

  .open-button:hover {
    background-color: var(--bg-tertiary);
    border-color: var(--accent-color);
  }

  .open-icon {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 20px;
    font-weight: 300;
    color: var(--accent-color);
    margin-right: 12px;
  }

  .open-text {
    flex: 1;
    text-align: left;
  }

  .open-shortcut {
    font-size: 12px;
    color: var(--text-secondary);
    background-color: var(--bg-tertiary);
    padding: 4px 8px;
    border-radius: 4px;
  }

  .recent-section {
    margin-top: 32px;
  }

  .section-title {
    font-size: 12px;
    font-weight: 500;
    text-transform: uppercase;
    color: var(--text-secondary);
    margin-bottom: 12px;
    letter-spacing: 0.5px;
  }

  .recent-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .empty-section {
    margin-top: 48px;
    text-align: center;
  }

  .empty-text {
    color: var(--text-secondary);
    font-size: 14px;
    margin-bottom: 8px;
  }

  .empty-hint {
    color: var(--text-secondary);
    font-size: 12px;
    opacity: 0.7;
  }
</style>
