<script lang="ts">
  import { AlertTriangle, Info } from 'lucide-svelte';
  import {
    errorMessage,
    extensionStatus,
    snapshot,
    workspaces,
  } from '../stores/status';
  import { openUrl } from './tauri';

  // System Settings deep-links. The `x-apple.systempreferences:` scheme is
  // documented for the Login Items / Extensions pane and the Privacy & Security
  // > Full Disk Access pane. Newer macOS still honours these URLs.
  const EXTENSIONS_URL =
    'x-apple.systempreferences:com.apple.LoginItems-Settings.extension';
  const FDA_URL =
    'x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles';

  $: showError = Boolean($errorMessage);
  $: showStale = !showError && Boolean($snapshot?.stale);
  $: showAllowExt =
    !showError &&
    !showStale &&
    Boolean($extensionStatus?.installed && !$extensionStatus?.enabled);
  // FDA hint: extension is allowed and the user has at least one workspace,
  // but status.json reports zero repos. The monitor writes status regardless
  // of FDA, so a zero-repo result with workspaces configured is the most
  // common signal that the extension cannot read those folders.
  $: showFda =
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

{#if showError}
  <div class="banner error">
    <AlertTriangle size={18} />
    <span>{$errorMessage}</span>
  </div>
{:else if showStale}
  <div class="banner warning">
    <AlertTriangle size={18} />
    <span>Monitor stopped</span>
    <code>launchctl load ~/Library/LaunchAgents/com.zaai.git-same.monitor.plist</code>
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
    <span>Grant Full Disk Access to Git-Same so badges can render on your repository folders.</span>
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

  .banner.info {
    color: var(--text);
  }

  code {
    color: var(--text);
    background: var(--panel-alt);
    padding: 3px 6px;
    border-radius: 6px;
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

  button:hover {
    background: var(--panel);
  }
</style>
