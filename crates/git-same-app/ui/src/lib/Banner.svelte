<script lang="ts">
  import { AlertTriangle } from 'lucide-svelte';
  import { errorMessage, snapshot } from '../stores/status';

  $: showError = Boolean($errorMessage);
  $: showStale = !showError && Boolean($snapshot?.stale);
</script>

{#if showError}
  <div class="banner error">
    <AlertTriangle size={18} />
    <span>{$errorMessage}</span>
  </div>
{:else if showStale}
  <div class="banner warning">
    <AlertTriangle size={18} />
    <span>Daemon stopped</span>
    <code>launchctl load ~/Library/LaunchAgents/com.zaai.git-same.daemon.plist</code>
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

  code {
    color: var(--text);
    background: var(--panel-alt);
    padding: 3px 6px;
    border-radius: 6px;
  }
</style>
