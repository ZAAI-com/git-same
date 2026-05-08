<script lang="ts">
  import { AlertTriangle, CheckCircle2, Info, X } from 'lucide-svelte';
  import {
    errorMessage,
    extensionStatus,
    snapshot,
    installMonitor,
    successMessage,
    syncProgress,
    workspaces,
  } from '../stores/status';
  import { openUrl } from './tauri';

  const EXTENSIONS_URL =
    'x-apple.systempreferences:com.apple.LoginItems-Settings.extension';
  const FDA_URL =
    'x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles';

  $: showSuccess = Boolean($successMessage);
  $: showError = !showSuccess && Boolean($errorMessage);
  $: showProgress = !showSuccess && !showError && Boolean($syncProgress);
  $: progressPercent =
    $syncProgress?.total && $syncProgress.total > 0
      ? Math.min(
          100,
          Math.round(($syncProgress.completed / $syncProgress.total) * 100),
        )
      : 0;
  $: progressCount =
    $syncProgress?.total && $syncProgress.total > 0
      ? `${Math.min($syncProgress.completed, $syncProgress.total)} / ${$syncProgress.total}`
      : '';
  $: progressMeta = [
    progressCount,
    $syncProgress?.failed ? `${$syncProgress.failed} failed` : '',
    $syncProgress?.skipped ? `${$syncProgress.skipped} skipped` : '',
  ]
    .filter(Boolean)
    .join(' · ');
  $: showStale = !showSuccess && !showError && !showProgress && Boolean($snapshot?.stale);
  $: showAllowExt =
    !showSuccess &&
    !showError &&
    !showStale &&
    Boolean($extensionStatus?.installed && !$extensionStatus?.enabled);
  $: showFda =
    !showSuccess &&
    !showError &&
    !showStale &&
    !showAllowExt &&
    Boolean($extensionStatus?.enabled) &&
    $workspaces.length > 0 &&
    ($snapshot?.status?.repos?.length ?? 0) === 0;

  function openExtensions() {
    void openUrl(EXTENSIONS_URL);
  }

  function openFullDiskAccess() {
    void openUrl(FDA_URL);
  }
</script>

{#if showSuccess}
  <div class="banner success">
    <CheckCircle2 size={18} />
    <span>{$successMessage}</span>
    <button type="button" class="dismiss" aria-label="Dismiss" on:click={() => successMessage.set('')}>
      <X size={15} />
    </button>
  </div>
{:else if showError}
  <div class="banner error">
    <AlertTriangle size={18} />
    <span>{$errorMessage}</span>
    <button type="button" class="dismiss" aria-label="Dismiss" on:click={() => errorMessage.set('')}>
      <X size={15} />
    </button>
  </div>
{:else if showProgress}
  <div class="banner progress">
    <Info size={18} />
    <div class="progress-copy">
      <span>{$syncProgress?.message}</span>
      {#if progressMeta}
        <small>{progressMeta}</small>
      {/if}
    </div>
    {#if $syncProgress?.total}
      <div class="progress-track" aria-hidden="true">
        <span style={`width: ${progressPercent}%`}></span>
      </div>
    {/if}
  </div>
{:else if showStale}
  <div class="banner warning">
    <AlertTriangle size={18} />
    <span>Monitor stopped; Finder badges may not update</span>
    <button type="button" on:click={installMonitor}>Install Monitor</button>
  </div>
{:else if showAllowExt}
  <div class="banner info">
    <Info size={18} />
    <span>Allow Finder badges in System Settings to see git status icons in Finder.</span>
    <button type="button" on:click={openExtensions}>Open Extensions</button>
  </div>
{:else if showFda}
  <div class="banner info">
    <Info size={18} />
    <span>Grant Full Disk Access to Git-Same so badges can render on repository folders.</span>
    <button type="button" on:click={openFullDiskAccess}>Grant Full Disk Access</button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 42px;
    margin-bottom: 16px;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: var(--panel);
    overflow-wrap: anywhere;
  }

  .banner.warning {
    color: var(--warning);
  }

  .banner.error {
    color: var(--danger);
  }

  .banner.success {
    color: var(--ok);
  }

  .banner.info,
  .banner.progress {
    color: var(--text);
  }

  .progress-copy {
    min-width: 0;
    display: grid;
    gap: 2px;
  }

  .progress-copy span,
  .progress-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .progress-copy small {
    color: var(--muted);
  }

  .progress-track {
    width: min(220px, 28vw);
    height: 6px;
    margin-left: auto;
    border-radius: 999px;
    background: var(--panel-alt);
    overflow: hidden;
  }

  .progress-track span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
  }

  button {
    margin-left: auto;
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid var(--line);
    background: var(--panel-alt);
    color: var(--text);
    cursor: pointer;
    font: inherit;
  }

  .dismiss {
    width: 28px;
    height: 28px;
    display: inline-grid;
    place-items: center;
    padding: 0;
  }

  button:hover {
    background: var(--hover);
  }

  @media (max-width: 640px) {
    .banner {
      align-items: flex-start;
      flex-wrap: wrap;
    }

    .progress-copy span,
    .progress-copy small {
      white-space: normal;
    }

    .progress-track {
      display: none;
    }
  }
</style>
