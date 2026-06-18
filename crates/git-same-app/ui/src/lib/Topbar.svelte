<script lang="ts">
  import { RefreshCw } from '@lucide/svelte';
  import { router } from 'svelte-spa-router';
  import {
    currentWorkspace,
    loading,
    snapshot,
    syncingId,
    refresh,
    startSyncCurrent,
  } from '../stores/status';

  $: isSettings = router.location === '/settings';
  $: title = isSettings ? 'Settings' : $currentWorkspace?.id ?? 'Dashboard';
  $: subtitle =
    $currentWorkspace?.root ??
    $snapshot?.status_path ??
    'No workspace selected';
  $: syncing = Boolean($syncingId);
</script>

<header class="topbar">
  <div>
    <h1>{title}</h1>
    <p>{subtitle}</p>
  </div>
  <div class="actions">
    <button
      class="icon-button"
      on:click={refresh}
      title="Refresh"
      aria-label="Refresh"
    >
      <RefreshCw size={18} class={$loading ? 'spinning' : ''} />
    </button>
    {#if !isSettings}
      <button
        class="primary"
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
  .topbar {
    display: flex;
    justify-content: space-between;
    gap: 18px;
    align-items: center;
    margin-bottom: 18px;
  }

  h1,
  p {
    margin: 0;
  }

  h1 {
    font-size: 24px;
    line-height: 1.2;
  }

  .topbar p {
    margin-top: 5px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .icon-button,
  .primary {
    border: 1px solid var(--line);
    border-radius: 8px;
    cursor: pointer;
  }

  .icon-button {
    width: 38px;
    height: 38px;
    display: inline-grid;
    place-items: center;
    background: var(--panel);
    color: var(--text);
  }

  .primary {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 13px;
    background: var(--accent);
    color: white;
    font-weight: 700;
  }

  .primary:disabled {
    cursor: not-allowed;
    opacity: 0.65;
  }
</style>
