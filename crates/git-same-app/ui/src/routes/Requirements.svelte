<script lang="ts">
  import { onMount } from 'svelte';
  import { AlertTriangle, CheckCircle2, ExternalLink, FilePlus2, RotateCcw } from 'lucide-svelte';
  import {
    createDefaultConfig,
    loadRequirements,
    requirements,
    requirementsLoading,
  } from '../stores/status';
  import { openUrl } from '../lib/tauri';

  const EXTENSIONS_URL =
    'x-apple.systempreferences:com.apple.LoginItems-Settings.extension';
  const FDA_URL =
    'x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles';

  onMount(() => {
    void loadRequirements();
  });

  $: passed = $requirements.filter((check) => check.passed).length;
  $: criticalFailures = $requirements.filter((check) => check.critical && !check.passed).length;

  function actionFor(name: string): 'config' | 'extensions' | 'fda' | null {
    if (name === 'Config file') return 'config';
    if (name === 'Finder extension') return 'extensions';
    if (name === 'Full Disk Access') return 'fda';
    return null;
  }

  function runAction(action: 'config' | 'extensions' | 'fda') {
    if (action === 'config') void createDefaultConfig().then(loadRequirements);
    if (action === 'extensions') void openUrl(EXTENSIONS_URL);
    if (action === 'fda') void openUrl(FDA_URL);
  }
</script>

<section class="requirements-screen">
  <section class="summary">
    <article>
      <strong>{passed}</strong>
      <span>Passing checks</span>
    </article>
    <article class:failed={criticalFailures > 0}>
      <strong>{criticalFailures}</strong>
      <span>Critical failures</span>
    </article>
    <button type="button" on:click={loadRequirements} disabled={$requirementsLoading}>
      <RotateCcw size={16} class={$requirementsLoading ? 'spinning' : ''} />
      <span>{$requirementsLoading ? 'Checking' : 'Recheck'}</span>
    </button>
  </section>

  <section class="panel">
    {#if $requirements.length === 0}
      <div class="empty">
        <span>Run checks to inspect system requirements.</span>
      </div>
    {:else}
      {#each $requirements as check}
        {@const action = actionFor(check.name)}
        <article class:failed={!check.passed} class="check-row">
          <span class={check.passed ? 'ok' : 'warn'}>
            {#if check.passed}<CheckCircle2 size={18} />{:else}<AlertTriangle size={18} />{/if}
          </span>
          <div>
            <strong>{check.name}</strong>
            <small>{check.message}</small>
            {#if check.suggestion && !check.passed}
              <p>{check.suggestion}</p>
            {/if}
          </div>
          <span class:critical={check.critical} class="severity">
            {check.critical ? 'Critical' : 'Optional'}
          </span>
          {#if action && !check.passed}
            <button type="button" on:click={() => runAction(action)}>
              {#if action === 'config'}<FilePlus2 size={15} />{:else}<ExternalLink size={15} />{/if}
              <span>{action === 'config' ? 'Create' : 'Open'}</span>
            </button>
          {/if}
        </article>
      {/each}
    {/if}
  </section>
</section>

<style>
  .requirements-screen {
    display: grid;
    gap: 16px;
  }

  .summary,
  .panel {
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: var(--shadow);
  }

  .summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(140px, 1fr)) auto;
    gap: 12px;
    align-items: center;
    padding: 14px;
  }

  .summary article {
    display: grid;
    gap: 4px;
  }

  .summary strong {
    font-size: 26px;
  }

  .summary span,
  small,
  p {
    color: var(--muted);
  }

  .summary article.failed strong {
    color: var(--danger);
  }

  .panel {
    overflow: hidden;
  }

  .check-row {
    min-height: 62px;
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr) 82px auto;
    align-items: center;
    gap: 12px;
    border-top: 1px solid var(--line);
    padding: 12px 14px;
  }

  .check-row:first-child {
    border-top: 0;
  }

  .check-row.failed {
    background: color-mix(in srgb, var(--warning) 5%, transparent);
  }

  .check-row strong,
  .check-row small,
  .check-row p {
    display: block;
    margin: 0;
    overflow-wrap: anywhere;
  }

  .ok,
  .warn {
    width: 32px;
    height: 32px;
    display: inline-grid;
    place-items: center;
    border-radius: 999px;
  }

  .ok {
    background: color-mix(in srgb, var(--ok) 14%, transparent);
    color: var(--ok);
  }

  .warn {
    background: color-mix(in srgb, var(--warning) 16%, transparent);
    color: var(--warning);
  }

  .severity {
    width: fit-content;
    border-radius: 999px;
    background: var(--panel-alt);
    color: var(--muted);
    font-size: 11px;
    font-weight: 800;
    padding: 4px 8px;
  }

  .severity.critical {
    color: var(--danger);
  }

  button {
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-alt);
    color: var(--text);
    cursor: pointer;
    padding: 0 12px;
    font-weight: 700;
    white-space: nowrap;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .empty {
    padding: 18px;
    color: var(--muted);
  }

  @media (max-width: 760px) {
    .summary,
    .check-row {
      grid-template-columns: 1fr;
      justify-items: start;
    }
  }
</style>
