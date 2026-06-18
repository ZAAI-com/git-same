<script lang="ts">
  import { RefreshCw, RotateCcw } from '@lucide/svelte';
  import { router } from 'svelte-spa-router';
  import {
    currentWorkspace,
    loadRequirements,
    loading,
    refresh,
    selectedWorkspaceId,
    NEW_WORKSPACE_ID,
    snapshot,
    startSyncCurrent,
    syncingId,
  } from '../stores/status';
  import { relativeTime } from './utils';

  $: route = router.location || '/dashboard';
  $: title = routeTitle(route);
  $: subtitle = routeSubtitle(route);
  $: showSync =
    route === '/dashboard' ||
    route === '/' ||
    route === '/workspace' ||
    route === '/workspace/screen';
  $: showRecheck = route === '/requirements';
  $: syncing = Boolean($syncingId);

  function routeTitle(route: string): string {
    switch (route) {
      case '/':
      case '/dashboard':
        return 'Dashboard';
      case '/finder-badges':
        return 'Finder Badges';
      case '/badge-browser':
        return 'Badge Browser';
      case '/workspace':
        return $currentWorkspace?.name ?? 'Workspace';
      case '/workspace/screen':
        return $selectedWorkspaceId === NEW_WORKSPACE_ID
          ? 'New Workspace'
          : 'Workspace screen';
      case '/settings':
        return 'Settings';
      case '/requirements':
        return 'Requirements';
      default:
        return 'Dashboard';
    }
  }

  function routeSubtitle(route: string): string {
    if (route === '/workspace' && $currentWorkspace) return $currentWorkspace.root;
    if (route === '/workspace') return 'Select a workspace from the sidebar';
    if (route === '/workspace/screen' && $currentWorkspace) return $currentWorkspace.root;
    if (route === '/workspace/screen' && $selectedWorkspaceId === NEW_WORKSPACE_ID) {
      return 'Create a portable .git-same workspace config';
    }
    if (route === '/finder-badges') return $snapshot?.status_path ?? 'Finder status file';
    if (route === '/badge-browser') return 'Repository badge states from the latest scan';
    if (route === '/settings') return 'Global config and Finder monitor defaults';
    if (route === '/requirements') return 'System, GitHub, monitor, and Finder checks';
    return `Last scan ${relativeTime($snapshot?.updated_at)}`;
  }
</script>

<header class="titlebar">
  <div class="title-copy">
    <h1>{title}</h1>
    <p>{subtitle}</p>
  </div>
  <div class="actions">
    <button
      class="icon-button"
      type="button"
      on:click={refresh}
      title="Refresh"
      aria-label="Refresh"
    >
      <RefreshCw size={18} class={$loading ? 'spinning' : ''} />
    </button>
    {#if showRecheck}
      <button class="secondary" type="button" on:click={loadRequirements}>
        <RotateCcw size={17} />
        <span>Recheck</span>
      </button>
    {/if}
    {#if showSync}
      <button
        class="primary"
        type="button"
        on:click={startSyncCurrent}
        disabled={!$currentWorkspace || syncing}
      >
        <RefreshCw size={17} class={syncing ? 'spinning' : ''} />
        <span>{syncing ? 'Syncing' : 'Sync'}</span>
      </button>
    {/if}
  </div>
</header>

<style>
  .titlebar {
    min-height: 62px;
    display: flex;
    justify-content: space-between;
    gap: 18px;
    align-items: center;
    border-bottom: 1px solid var(--line);
    background: color-mix(in srgb, var(--bg) 88%, transparent);
    padding: 14px 22px;
  }

  .title-copy {
    min-width: 0;
  }

  h1,
  p {
    margin: 0;
  }

  h1 {
    font-size: 22px;
    line-height: 1.2;
  }

  p {
    margin-top: 4px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .icon-button,
  .primary,
  .secondary {
    height: 36px;
    border: 1px solid var(--line);
    border-radius: 7px;
    cursor: pointer;
  }

  .icon-button {
    width: 36px;
    display: inline-grid;
    place-items: center;
    background: var(--panel);
    color: var(--text);
  }

  .primary,
  .secondary {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 12px;
    font-weight: 700;
  }

  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .secondary {
    background: var(--panel);
    color: var(--text);
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.58;
  }

  @media (max-width: 720px) {
    .titlebar {
      align-items: flex-start;
      flex-direction: column;
    }

    p {
      white-space: normal;
    }
  }
</style>
