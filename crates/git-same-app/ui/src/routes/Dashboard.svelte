<script lang="ts">
  import { AlertTriangle, CheckCircle2, FolderGit2, RefreshCw } from '@lucide/svelte';
  import BadgeChip from '../lib/BadgeChip.svelte';
  import EmptyState from '../lib/EmptyState.svelte';
  import {
    currentWorkspace,
    extensionStatus,
    selectedWorkspaceId,
    snapshot,
    startSyncCurrent,
    syncingId,
    workspaces,
  } from '../stores/status';
  import {
    formatCount,
    isHighRiskRepo,
    relativeTime,
    repoChangeCount,
    repoName,
    summarize,
  } from '../lib/utils';
  import type { WorkspaceSummary } from '../lib/types';

  $: repos = $snapshot?.status?.repos ?? [];
  $: counts = summarize(repos);
  $: highRiskRepos = repos.filter(isHighRiskRepo).slice(0, 8);
  $: monitorOk = Boolean($snapshot && !$snapshot.stale);
  $: extensionOk = Boolean($extensionStatus?.installed && $extensionStatus?.enabled);

  function reposFor(workspace: WorkspaceSummary) {
    return repos.filter(
      (repo) =>
        repo.workspace === workspace.id ||
        repo.workspace === workspace.name ||
        repo.path.startsWith(workspace.root),
    );
  }

  function workspaceCounts(workspace: WorkspaceSummary) {
    return summarize(reposFor(workspace));
  }

  async function syncWorkspace(workspace: WorkspaceSummary) {
    selectedWorkspaceId.set(workspace.id);
    await startSyncCurrent();
  }
</script>

<section class="dashboard">
  <div class="health-grid">
    <article class="health-card">
      <span class={monitorOk ? 'ok' : 'warn'}>
        {#if monitorOk}<CheckCircle2 size={18} />{:else}<AlertTriangle size={18} />{/if}
      </span>
      <div>
        <strong>{monitorOk ? 'Monitor running' : 'Monitor needs attention'}</strong>
        <p>Last scan {relativeTime($snapshot?.updated_at)}</p>
      </div>
    </article>
    <article class="health-card">
      <span class={extensionOk ? 'ok' : 'warn'}>
        {#if extensionOk}<CheckCircle2 size={18} />{:else}<AlertTriangle size={18} />{/if}
      </span>
      <div>
        <strong>{extensionOk ? 'Finder extension enabled' : 'Finder extension not ready'}</strong>
        <p>{$extensionStatus?.installed ? 'Installed' : 'Not installed'} · {$extensionStatus?.enabled ? 'Enabled' : 'Disabled'}</p>
      </div>
    </article>
    <article class="metric">
      <strong>{$workspaces.length}</strong>
      <span>{formatCount($workspaces.length, 'workspace')}</span>
    </article>
    <article class="metric">
      <strong>{counts.total}</strong>
      <span>{formatCount(counts.total, 'repo')}</span>
    </article>
  </div>

  <section class="panel">
    <div class="panel-head">
      <h2>Badge Distribution</h2>
      <p>{counts.red + counts.orange} repos need review</p>
    </div>
    <div class="badge-strip">
      <BadgeChip badge="green" count={counts.green} />
      <BadgeChip badge="blue" count={counts.blue} />
      <BadgeChip badge="orange" count={counts.orange} />
      <BadgeChip badge="red" count={counts.red} />
      <BadgeChip badge="gray" count={counts.gray} />
    </div>
  </section>

  <div class="two-column">
    <section class="panel">
      <div class="panel-head">
        <h2>Workspaces</h2>
        <p>{$currentWorkspace?.name ?? 'No workspace selected'}</p>
      </div>
      {#if $workspaces.length === 0}
        <EmptyState
          title="No workspaces configured"
          detail="Create a workspace from the sidebar to start mirroring repositories locally."
        />
      {:else}
        <div class="workspace-list">
          {#each $workspaces as workspace}
            {@const summary = workspaceCounts(workspace)}
            <article class="workspace-row">
              <FolderGit2 size={18} />
              <div>
                <strong>{workspace.name}</strong>
                <small title={workspace.root}>{workspace.root}</small>
              </div>
              <BadgeChip badge={summary.red > 0 ? 'red' : summary.orange > 0 ? 'orange' : 'green'} count={summary.total} />
              <button
                type="button"
                on:click={() => syncWorkspace(workspace)}
                disabled={$syncingId === workspace.id}
              >
                <RefreshCw size={15} class={$syncingId === workspace.id ? 'spinning' : ''} />
                <span>{$syncingId === workspace.id ? 'Syncing' : 'Sync'}</span>
              </button>
            </article>
          {/each}
        </div>
      {/if}
    </section>

    <section class="panel">
      <div class="panel-head">
        <h2>Needs Attention</h2>
        <p>{formatCount(highRiskRepos.length, 'repo')}</p>
      </div>
      {#if highRiskRepos.length === 0}
        <EmptyState title="No high-risk repositories" detail="Red badges and scan errors will appear here." />
      {:else}
        <div class="risk-list">
          {#each highRiskRepos as repo}
            <article class="risk-row">
              <div>
                <strong>{repoName(repo.path)}</strong>
                <small>{repo.org ?? repo.workspace ?? repo.path}</small>
              </div>
              <BadgeChip badge={repo.badge} />
              <span>{repoChangeCount(repo)} changes</span>
              <span>{repo.ahead} ahead / {repo.behind} behind</span>
            </article>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</section>

<style>
  .dashboard {
    display: grid;
    gap: 16px;
  }

  .health-grid {
    display: grid;
    grid-template-columns: minmax(210px, 1.4fr) minmax(210px, 1.4fr) repeat(2, minmax(120px, 0.8fr));
    gap: 12px;
  }

  .health-card,
  .metric,
  .panel {
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: var(--shadow);
  }

  .health-card {
    min-height: 78px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px;
  }

  .health-card strong,
  .metric strong {
    display: block;
    font-size: 15px;
  }

  .health-card p,
  .metric span,
  .panel-head p,
  small {
    margin: 0;
    color: var(--muted);
  }

  .health-card span.ok,
  .health-card span.warn {
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

  .metric {
    min-height: 78px;
    display: grid;
    align-content: center;
    gap: 4px;
    padding: 14px;
  }

  .metric strong {
    font-size: 26px;
  }

  .panel {
    padding: 16px;
  }

  .panel-head {
    display: flex;
    justify-content: space-between;
    gap: 14px;
    align-items: baseline;
    margin-bottom: 12px;
  }

  h2 {
    margin: 0;
    font-size: 15px;
  }

  .badge-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .two-column {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 16px;
  }

  .workspace-list,
  .risk-list {
    display: grid;
    gap: 8px;
  }

  .workspace-row,
  .risk-row {
    min-height: 52px;
    display: grid;
    align-items: center;
    gap: 10px;
    border-top: 1px solid var(--line);
    padding: 10px 0 0;
  }

  .workspace-row {
    grid-template-columns: 24px minmax(0, 1fr) auto auto;
  }

  .risk-row {
    grid-template-columns: minmax(0, 1fr) auto auto auto;
  }

  .workspace-row strong,
  .workspace-row small,
  .risk-row strong,
  .risk-row small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workspace-row button {
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

  .workspace-row button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .risk-row > span {
    color: var(--muted);
    font-size: 13px;
  }

  @media (max-width: 1080px) {
    .health-grid,
    .two-column {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 700px) {
    .workspace-row,
    .risk-row {
      grid-template-columns: 1fr;
      justify-items: start;
    }
  }
</style>
