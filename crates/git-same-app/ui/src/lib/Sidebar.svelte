<script lang="ts">
  import {
    BadgeIcon,
    CircleDot,
    FolderGit2,
    Gauge,
    Plus,
    Settings,
    Wrench,
  } from 'lucide-svelte';
  import { location, push } from 'svelte-spa-router';
  import BrandLogo from './BrandLogo.svelte';
  import NavItem from './NavItem.svelte';
  import {
    NEW_WORKSPACE_ID,
    selectedWorkspaceId,
    workspaces,
  } from '../stores/status';

  $: path = $location || '/dashboard';
  $: activeTop = path === '/' ? '/dashboard' : path;

  function newWorkspace() {
    selectedWorkspaceId.set(NEW_WORKSPACE_ID);
    void push('/workspace/screen');
  }

  function openWorkspace(id: string) {
    selectedWorkspaceId.set(id);
    void push('/workspace');
  }
</script>

<aside class="sidebar">
  <div class="brand-wrap">
    <BrandLogo />
  </div>

  <nav class="nav primary" aria-label="General">
    <NavItem href="/dashboard" label="Dashboard" active={activeTop === '/dashboard'}>
      <Gauge slot="icon" size={17} />
    </NavItem>
    <NavItem href="/finder-badges" label="Finder Badges" active={activeTop === '/finder-badges'}>
      <BadgeIcon slot="icon" size={17} />
    </NavItem>
    <NavItem href="/badge-browser" label="Badge Browser" active={activeTop === '/badge-browser'}>
      <CircleDot slot="icon" size={17} />
    </NavItem>
  </nav>

  <section class="workspace-section" aria-label="Workspaces">
    <div class="section-head">
      <span>Workspaces</span>
      <button type="button" title="New workspace" aria-label="New workspace" on:click={newWorkspace}>
        <Plus size={15} />
      </button>
    </div>

    <div class="workspace-list">
      {#if $workspaces.length === 0}
        <button class="workspace empty" type="button" on:click={newWorkspace}>
          <FolderGit2 size={16} />
          <span>Add workspace</span>
        </button>
      {:else}
        {#each $workspaces as workspace}
          <button
            class:active={activeTop === '/workspace' && $selectedWorkspaceId === workspace.id}
            class="workspace"
            type="button"
            title={workspace.root}
            on:click={() => openWorkspace(workspace.id)}
          >
            <FolderGit2 size={16} />
            <span>{workspace.name}</span>
            {#if workspace.default}
              <small>Default</small>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </section>

  <nav class="nav bottom" aria-label="Utility">
    <NavItem href="/settings" label="Settings" active={activeTop === '/settings'}>
      <Settings slot="icon" size={17} />
    </NavItem>
    <NavItem href="/requirements" label="Requirements" active={activeTop === '/requirements'}>
      <Wrench slot="icon" size={17} />
    </NavItem>
  </nav>
</aside>

<style>
  .sidebar {
    min-height: 100vh;
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    gap: 16px;
    border-right: 1px solid var(--line);
    background: var(--sidebar);
    padding: 16px 12px;
  }

  .brand-wrap {
    padding: 8px 8px 10px;
  }

  .nav {
    display: grid;
    gap: 4px;
  }

  .workspace-section {
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 6px;
  }

  .section-head {
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 4px 0 9px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  .section-head button {
    width: 25px;
    height: 25px;
    display: inline-grid;
    place-items: center;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .section-head button:hover {
    border-color: var(--line);
    background: var(--hover);
    color: var(--text);
  }

  .workspace-list {
    min-height: 0;
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 4px;
  }

  .workspace {
    width: 100%;
    min-height: 32px;
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    padding: 0 9px;
    text-align: left;
  }

  .workspace:hover {
    background: var(--hover);
  }

  .workspace.active {
    background: var(--selected);
    color: var(--accent);
    font-weight: 700;
  }

  .workspace.empty {
    color: var(--muted);
  }

  .workspace span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workspace small {
    border-radius: 999px;
    background: var(--panel-alt);
    color: var(--muted);
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
  }

  .bottom {
    padding-top: 10px;
    border-top: 1px solid var(--line);
  }

  @media (max-width: 860px) {
    .sidebar {
      min-height: auto;
      border-right: 0;
      border-bottom: 1px solid var(--line);
      grid-template-rows: auto auto auto auto;
    }

    .workspace-list {
      max-height: 128px;
    }
  }
</style>
