<script lang="ts">
  import {
    AlertTriangle,
    CheckCircle2,
    ExternalLink,
    FolderSearch,
    Info,
    Play,
  } from '@lucide/svelte';
  import BadgeChip from '../lib/BadgeChip.svelte';
  import EmptyState from '../lib/EmptyState.svelte';
  import MonitorPanel from '../lib/MonitorPanel.svelte';
  import { presentMonitor } from '../lib/monitorPresentation';
  import { monitorStatus } from '../stores/monitor';
  import {
    enableExtension,
    extensionStatus,
    fullDiskAccess,
    installMonitor,
    restartMonitor,
    snapshot,
  } from '../stores/status';
  import { openUrl } from '../lib/tauri';
  import { EXTENSIONS_URL, FDA_URL } from '../lib/systemSettings';
  import { relativeTime } from '../lib/utils';
  import type { FullDiskAccessDto } from '../lib/types';

  type SetupRow = {
    label: string;
    passed: boolean;
    detail: string;
    action: string | null;
    disabled?: boolean;
    hint?: string;
  };

  // Set once an in-app enable attempt left the extension disabled (the
  // pluginkit election did not stick); the row then falls back to the
  // System Settings pane.
  let enableFallback = false;

  $: status = $snapshot?.status ?? null;
  $: roots = status?.monitored_roots ?? [];
  $: monitorView = presentMonitor($monitorStatus);
  $: fdaGranted = Boolean($fullDiskAccess?.granted);
  $: extensionEnabled = Boolean($extensionStatus?.enabled);
  $: if (extensionEnabled) enableFallback = false;
  // Order matters: the monitor must run so it can report its own Full Disk
  // Access, the grant must exist before badges make sense, and only then is
  // enabling the extension offered.
  $: setupRows = [
    {
      // Controls live in the Monitor panel above; an intentional stop is
      // listed here without being presented as something broken to fix.
      label: 'Monitor',
      passed:
        monitorView.healthy ||
        ($monitorStatus?.state === 'disabled' && !$monitorStatus?.autostart),
      detail: $monitorStatus?.state === 'running' && $snapshot?.updated_at
        ? `Running. Badge data updated ${relativeTime($snapshot.updated_at)}`
        : monitorView.title,
      action: null,
    },
    {
      label: 'Full Disk Access',
      passed: fdaGranted,
      detail: fdaDetail($fullDiskAccess),
      action: FDA_URL,
    },
    {
      label: 'Finder extension installed',
      passed: Boolean($extensionStatus?.installed),
      detail: $extensionStatus?.installed ? 'GitSameBadges.appex is registered' : 'Extension not found',
      action: EXTENSIONS_URL,
    },
    {
      label: 'Finder extension enabled',
      passed: extensionEnabled,
      detail: extensionEnabled
        ? 'Finder can request badges'
        : !fdaGranted
          ? 'Grant Full Disk Access first'
          : enableFallback
            ? 'Enable Git-Same Badges in System Settings'
            : 'Enable Git-Same Badges to show status in Finder',
      action: enableFallback ? EXTENSIONS_URL : 'enable-extension',
      disabled: !fdaGranted,
      hint: !fdaGranted ? 'Grant Full Disk Access first' : undefined,
    },
  ] satisfies SetupRow[];

  function fdaDetail(fda: FullDiskAccessDto | null): string {
    if (!fda) return 'Could not be determined';
    if (fda.granted) return 'Granted to Git-Same';
    if (fda.host === 'granted' && fda.monitor === false) {
      return 'Granted to the app; the monitor restarts to pick up the grant';
    }
    if (fda.host === 'denied' || fda.monitor === false) {
      return 'Not granted. Grant it in System Settings, then quit and reopen Git-Same';
    }
    if (fda.host === 'not_applicable') return 'Not applicable on this platform';
    return 'Could not be determined';
  }

  function actionLabel(action: string): string {
    if (action === 'install-monitor') return 'Install';
    if (action === 'restart-monitor') return 'Restart';
    if (action === 'enable-extension') return 'Enable badges';
    return 'Open';
  }

  async function runAction(action: string | null) {
    if (!action) return;
    if (action === 'install-monitor') await installMonitor();
    else if (action === 'restart-monitor') await restartMonitor();
    else if (action === 'enable-extension') {
      await enableExtension();
      if (!$extensionStatus?.enabled && $fullDiskAccess?.granted) enableFallback = true;
    } else await openUrl(action);
  }
</script>

<section class="finder-screen">
  {#if !monitorView.banner}
    <MonitorPanel />
  {/if}
  <section class="panel">
    <div class="panel-head">
      <h2>Setup Checklist</h2>
      <p>{setupRows.filter((row) => row.passed).length} / {setupRows.length} ready</p>
    </div>
    <div class="check-list">
      {#each setupRows as row}
        <article class:failed={!row.passed} class="check-row">
          <span class={row.passed ? 'ok' : 'warn'}>
            {#if row.passed}<CheckCircle2 size={18} />{:else}<AlertTriangle size={18} />{/if}
          </span>
          <div>
            <strong>{row.label}</strong>
            <small>{row.detail}</small>
          </div>
          {#if row.action && !row.passed}
            <button
              type="button"
              disabled={row.disabled}
              title={row.hint ?? ''}
              on:click={() => void runAction(row.action)}
            >
              {#if row.action.includes('monitor') || row.action === 'enable-extension'}<Play size={15} />{:else}<ExternalLink size={15} />{/if}
              <span>{actionLabel(row.action)}</span>
            </button>
          {/if}
        </article>
      {/each}
    </div>
  </section>

  <div class="two-column">
    <section class="panel">
      <div class="panel-head">
        <h2>Badge Legend</h2>
        <p>Finder folder overlays</p>
      </div>
      <div class="legend">
        <article><BadgeChip badge="green" /><span>Clean, synced, and safe to mirror.</span></article>
        <article><BadgeChip badge="blue" /><span>Synced, but important ignored local files exist.</span></article>
        <article><BadgeChip badge="orange" /><span>Main is clean, but another branch or worktree diverges.</span></article>
        <article><BadgeChip badge="red" /><span>Uncommitted work, untracked files, or unpushed commits.</span></article>
        <article><BadgeChip badge="gray" /><span>Ambient repo pending deeper classification.</span></article>
      </div>
    </section>

    <section class="panel">
      <div class="panel-head">
        <h2>Status File</h2>
        <p>Monitor output</p>
      </div>
      <dl class="details">
        <div>
          <dt>Path</dt>
          <dd>{$snapshot?.status_path ?? 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Last update</dt>
          <dd>{relativeTime($snapshot?.updated_at)}</dd>
        </div>
        <div>
          <dt>Monitor PID</dt>
          <dd>{status?.daemon_pid ?? 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Repos visible</dt>
          <dd>{status?.repos.length ?? 0}</dd>
        </div>
      </dl>
    </section>
  </div>

  <section class="panel">
    <div class="panel-head">
      <h2>Monitored Roots</h2>
      <p>{roots.length} paths</p>
    </div>
    {#if roots.length === 0}
      <EmptyState title="No monitored roots" detail="Workspace roots and ambient scan roots appear here after the monitor writes status." />
    {:else}
      <div class="root-list">
        {#each roots as root}
          <article>
            <FolderSearch size={16} />
            <span>{root}</span>
          </article>
        {/each}
      </div>
    {/if}
  </section>

  <section class="note">
    <Info size={18} />
    <span>Finder badges update from the monitor status file. The app never deletes repositories when changing badge settings.</span>
  </section>
</section>

<style>
  .finder-screen {
    display: grid;
    gap: 16px;
  }

  .panel,
  .note {
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: var(--shadow);
    padding: 16px;
  }

  .panel-head {
    display: flex;
    justify-content: space-between;
    gap: 14px;
    align-items: baseline;
    margin-bottom: 12px;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    font-size: 15px;
  }

  .panel-head p,
  small,
  .legend span,
  .note,
  dt {
    color: var(--muted);
  }

  .check-list,
  .legend,
  .root-list,
  .details {
    display: grid;
    gap: 10px;
  }

  .check-row {
    min-height: 48px;
    display: grid;
    grid-template-columns: 30px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    border-top: 1px solid var(--line);
    padding-top: 10px;
  }

  .check-row:first-child {
    border-top: 0;
    padding-top: 0;
  }

  .check-row strong,
  .check-row small {
    display: block;
  }

  .ok,
  .warn {
    width: 30px;
    height: 30px;
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

  button {
    height: 32px;
    display: flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-alt);
    color: var(--text);
    cursor: pointer;
    padding: 0 10px;
    font-weight: 700;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .two-column {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 16px;
  }

  .legend article,
  .root-list article,
  .note {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .root-list article {
    min-width: 0;
    min-height: 34px;
  }

  .root-list span,
  dd {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .details {
    margin: 0;
  }

  .details div {
    display: grid;
    gap: 3px;
    border-top: 1px solid var(--line);
    padding-top: 9px;
  }

  .details div:first-child {
    border-top: 0;
    padding-top: 0;
  }

  dt {
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
  }

  dd {
    margin: 0;
  }

  @media (max-width: 980px) {
    .two-column {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .check-row {
      grid-template-columns: 30px minmax(0, 1fr);
    }

    .check-row button {
      grid-column: 2;
      width: fit-content;
    }
  }
</style>
