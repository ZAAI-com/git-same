<script lang="ts">
  import { AlertTriangle, CheckCircle2, Info } from '@lucide/svelte';
  import { monitorBusy, monitorError, monitorStatus, runMonitorAction } from '../stores/monitor';
  import { actionLabel, presentMonitor } from './monitorPresentation';

  $: view = presentMonitor($monitorStatus);
</script>

<section class="monitor-panel {view.tone}" aria-live="polite">
  <div class="icon">
    {#if view.tone === 'ok'}
      <CheckCircle2 size={18} />
    {:else if view.tone === 'info'}
      <Info size={18} />
    {:else}
      <AlertTriangle size={18} />
    {/if}
  </div>
  <div class="copy">
    <strong>{view.title}</strong>
    {#if $monitorError}
      <small class="error-text">{$monitorError}</small>
    {:else if view.detail}
      <small>{view.detail}</small>
    {/if}
  </div>
  <div class="actions">
    {#each view.actions as action, index}
      <button
        type="button"
        class:primary={index === 0 && !view.healthy}
        disabled={$monitorBusy}
        on:click={() => runMonitorAction(action)}
      >
        {actionLabel(action, $monitorStatus)}
      </button>
    {/each}
  </div>
</section>

<style>
  .monitor-panel {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    margin-bottom: 16px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
  }

  .icon {
    display: flex;
    flex: none;
  }

  .ok .icon {
    color: var(--ok, #2e9e5b);
  }

  .warning .icon,
  .error .icon {
    color: var(--warn, #c98a12);
  }

  .copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .copy small {
    color: var(--muted);
    overflow-wrap: anywhere;
  }

  .copy .error-text {
    color: var(--danger, #c0392b);
  }

  .actions {
    display: flex;
    gap: 8px;
    flex: none;
  }
</style>
