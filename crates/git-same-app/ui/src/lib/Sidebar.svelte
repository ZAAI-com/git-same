<script lang="ts">
  import { FolderGit2, Settings, ShieldCheck } from 'lucide-svelte';
  import { location, push } from 'svelte-spa-router';
  import { selectedWorkspaceId, workspaces } from '../stores/status';

  $: isSettings = $location === '/settings';
  $: isDashboard = !isSettings;
</script>

<aside class="sidebar">
  <div class="brand">
    <FolderGit2 size={22} />
    <span>git-Same</span>
  </div>

  <nav class="nav">
    <button class:active={isDashboard} on:click={() => push('/')}>
      <ShieldCheck size={17} />
      <span>Dashboard</span>
    </button>
    <button class:active={isSettings} on:click={() => push('/settings')}>
      <Settings size={17} />
      <span>Settings</span>
    </button>
  </nav>

  <div class="workspace-list">
    {#each $workspaces as workspace}
      <button
        class:active={$selectedWorkspaceId === workspace.id}
        on:click={() => selectedWorkspaceId.set(workspace.id)}
        title={workspace.root}
      >
        <span>{workspace.id}</span>
        <small>{workspace.provider}</small>
      </button>
    {/each}
  </div>
</aside>

<style>
  .sidebar {
    border-right: 1px solid var(--line);
    background: var(--panel);
    padding: 18px 14px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    font-size: 18px;
    font-weight: 700;
  }

  .nav {
    display: grid;
    gap: 6px;
    margin: 24px 0;
  }

  .nav button {
    display: flex;
    align-items: center;
    width: 100%;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    text-align: left;
    gap: 9px;
    height: 38px;
    padding: 0 10px;
  }

  .nav button.active,
  .workspace-list button.active {
    background: var(--panel-alt);
  }

  .workspace-list {
    display: grid;
    gap: 8px;
  }

  .workspace-list button {
    display: grid;
    width: 100%;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    text-align: left;
    gap: 3px;
    min-height: 58px;
    padding: 10px;
  }

  .workspace-list span,
  .workspace-list small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workspace-list small {
    color: var(--muted);
  }

  @media (max-width: 860px) {
    .sidebar {
      border-right: 0;
      border-bottom: 1px solid var(--line);
    }
  }
</style>
