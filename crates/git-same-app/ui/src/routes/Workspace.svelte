<script lang="ts">
  import { CircleDot, ExternalLink, FolderGit2, Github, Settings } from 'lucide-svelte';
  import { location, push } from 'svelte-spa-router';
  import BadgeChip from '../lib/BadgeChip.svelte';
  import EmptyState from '../lib/EmptyState.svelte';
  import {
    NEW_WORKSPACE_ID,
    currentWorkspace,
    loadCurrentWorkspaceStructure,
    selectedWorkspaceId,
    snapshot,
    workspaceStructure,
    workspaceStructureLoading,
  } from '../stores/status';
  import { badgeLabel, formatCount, repoName } from '../lib/utils';
  import type { FinderRepoStatus, WorkspaceStructureRepoDto } from '../lib/types';

  type PairEntry = {
    remote?: WorkspaceStructureRepoDto;
    local?: FinderRepoStatus;
    name: string;
  };
  type OwnerGroup = { owner: string; entries: PairEntry[] };

  let loadedWorkspaceId = '';

  $: route = $location || '/workspace';
  $: selectedId = $selectedWorkspaceId;
  $: if (selectedId && selectedId !== NEW_WORKSPACE_ID && selectedId !== loadedWorkspaceId) {
    loadedWorkspaceId = selectedId;
    void loadCurrentWorkspaceStructure();
  }

  $: localRepos = workspaceRepos();
  $: remoteRepos = $workspaceStructure?.repos ?? [];
  $: pairedGroups = buildPairs(remoteRepos, localRepos);
  $: missingCount = pairedGroups
    .flatMap((group) => group.entries)
    .filter((entry) => entry.remote && !entry.remote.local_exists).length;
  $: localOnlyCount = pairedGroups
    .flatMap((group) => group.entries)
    .filter((entry) => !entry.remote).length;

  function workspaceRepos(): FinderRepoStatus[] {
    const workspace = $currentWorkspace;
    const repos = $snapshot?.status?.repos ?? [];
    if (!workspace) return [];
    return repos.filter(
      (repo) =>
        repo.workspace === workspace.id ||
        repo.workspace === workspace.name ||
        repo.path.startsWith(workspace.root),
    );
  }

  function buildPairs(
    remotes: WorkspaceStructureRepoDto[],
    locals: FinderRepoStatus[],
  ): OwnerGroup[] {
    const byPath = new Map<string, PairEntry>();
    for (const remote of remotes) {
      const key = normalizePath(remote.local_path);
      if (!byPath.has(key)) byPath.set(key, { remote, name: remote.name });
    }
    for (const local of locals) {
      const key = normalizePath(local.path);
      const slot = byPath.get(key);
      if (slot && !slot.local) {
        slot.local = local;
      } else if (!slot) {
        byPath.set(key, { local, name: repoName(local.path) });
      }
    }
    const groups = new Map<string, { display: string; entries: PairEntry[] }>();
    for (const entry of byPath.values()) {
      const rawOwner =
        entry.remote?.owner ?? entry.local?.org ?? ownerFromPath(entry.local?.path ?? '');
      const key = rawOwner.toLowerCase();
      const slot = groups.get(key) ?? { display: rawOwner, entries: [] };
      if (entry.remote) slot.display = entry.remote.owner;
      slot.entries.push(entry);
      groups.set(key, slot);
    }
    return [...groups.values()]
      .sort((left, right) => left.display.localeCompare(right.display))
      .map((group) => ({
        owner: group.display,
        entries: group.entries.sort((left, right) => left.name.localeCompare(right.name)),
      }));
  }

  function ownerFromPath(path: string): string {
    const workspace = $currentWorkspace;
    if (!path) return workspace?.name ?? 'Local';
    if (workspace && path.startsWith(workspace.root)) {
      const relative = path.slice(workspace.root.length).split('/').filter(Boolean);
      return relative.length > 1 ? relative[0] : workspace.name;
    }
    const parts = path.split('/').filter(Boolean);
    return parts.at(-2) ?? 'Local';
  }

  function normalizePath(path: string): string {
    return path.replace(/\/+$/, '');
  }

  function entryKey(entry: PairEntry): string {
    return entry.remote?.full_name ?? entry.local?.path ?? entry.name;
  }

  function sourceLabel(): string {
    if (!$workspaceStructure) return 'No structure loaded';
    if ($workspaceStructure.source === 'cache') return 'GitHub structure from cache';
    if ($workspaceStructure.source === 'remote') return 'GitHub structure from remote';
    return 'GitHub structure unavailable';
  }
</script>

<section class="workspace-overview">
  <nav class="subnav" aria-label="Workspace screens">
    <button class:active={route === '/workspace'} type="button" on:click={() => push('/workspace')}>
      <CircleDot size={15} />
      <span>Overview</span>
    </button>
    <button
      class:active={route === '/workspace/screen'}
      type="button"
      on:click={() => push('/workspace/screen')}
    >
      <Settings size={15} />
      <span>Workspace screen</span>
    </button>
  </nav>

  {#if !$currentWorkspace}
    <EmptyState
      title="No workspace selected"
      detail="Select a workspace from the sidebar or create a new workspace."
    >
      <button type="button" on:click={() => {
        selectedWorkspaceId.set(NEW_WORKSPACE_ID);
        void push('/workspace/screen');
      }}>Create Workspace</button>
    </EmptyState>
  {:else}
    <section class="summary-grid">
      <article>
        <strong>{formatCount(remoteRepos.length, 'GitHub repo')}</strong>
        <span>{sourceLabel()}</span>
      </article>
      <article>
        <strong>{formatCount(localRepos.length, 'local repo')}</strong>
        <span>{$currentWorkspace.root}</span>
      </article>
      <article>
        <strong>{missingCount}</strong>
        <span>Missing locally</span>
      </article>
      <article>
        <strong>{localOnlyCount}</strong>
        <span>Local only</span>
      </article>
    </section>

    {#if $workspaceStructure?.error}
      <div class="inline-warning">{$workspaceStructure.error}</div>
    {/if}

    <section class="panel">
      <div class="panel-head">
        <div class="panel-head-text">
          <h2>Repositories</h2>
          <p>{$workspaceStructure?.host ?? 'github.com'} · {$currentWorkspace.root}</p>
        </div>
        <div class="head-icons">
          <Github size={19} />
          <FolderGit2 size={19} />
        </div>
      </div>

      {#if $workspaceStructureLoading && remoteRepos.length === 0 && localRepos.length === 0}
        <div class="table-empty">Loading workspace…</div>
      {:else if pairedGroups.length === 0}
        <EmptyState
          title="No repositories"
          detail="Run sync or refresh discovery to populate this workspace."
        />
      {:else}
        <div class="table">
          <span class="cell-header gh-side"></span>
          <span class="cell-header gh-side label">
            <Github size={15} />
            <strong>GitHub</strong>
          </span>
          <span class="cell-header gh-side"></span>
          <span class="cell-header gh-side"></span>
          <span class="cell-header divider" aria-hidden="true"></span>
          <span class="cell-header local-side"></span>
          <span class="cell-header local-side label">
            <FolderGit2 size={15} />
            <strong>Local</strong>
          </span>
          <span class="cell-header local-side"></span>
          <span class="cell-header local-side"></span>

          {#each pairedGroups as group (group.owner)}
            <div class="owner-row">
              <span class="branch">├─</span>
              <strong>{group.owner}</strong>
            </div>

            {#each group.entries as entry (entryKey(entry))}
              {#if entry.remote}
                <a
                  class="cell branch"
                  class:missing={!entry.remote.local_exists}
                  href={entry.remote.url}
                  target="_blank"
                  rel="noreferrer"
                >│ ├─</a>
                <a
                  class="cell name"
                  class:missing={!entry.remote.local_exists}
                  href={entry.remote.url}
                  target="_blank"
                  rel="noreferrer"
                >{entry.remote.name}</a>
                <a
                  class="cell meta"
                  class:missing={!entry.remote.local_exists}
                  href={entry.remote.url}
                  target="_blank"
                  rel="noreferrer"
                ><small>{entry.remote.local_exists ? 'mirrored' : 'missing'}</small></a>
                <a
                  class="cell icon"
                  class:missing={!entry.remote.local_exists}
                  href={entry.remote.url}
                  target="_blank"
                  rel="noreferrer"
                ><ExternalLink size={13} /></a>
              {:else}
                <span class="cell empty"></span>
                <span class="cell empty"></span>
                <span class="cell empty"></span>
                <span class="cell empty"></span>
              {/if}

              <span class="cell divider" aria-hidden="true"></span>

              {#if entry.local}
                <span class="cell branch" class:local-only={!entry.remote}>│ ├─</span>
                <span class="cell name" class:local-only={!entry.remote}>{repoName(entry.local.path)}</span>
                <span class="cell badge"><BadgeChip badge={entry.local.badge} /></span>
                <span class="cell meta"><small>{badgeLabel(entry.local.badge)}</small></span>
              {:else}
                <span class="cell empty"></span>
                <span class="cell empty"></span>
                <span class="cell empty"></span>
                <span class="cell empty"></span>
              {/if}
            {/each}
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</section>

<style>
  .workspace-overview {
    display: grid;
    gap: 14px;
  }

  .subnav {
    width: fit-content;
    display: inline-flex;
    gap: 4px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    padding: 4px;
  }

  .subnav button {
    min-height: 30px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    padding: 0 10px;
    font-weight: 700;
  }

  .subnav button.active {
    background: var(--selected);
    color: var(--accent);
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(120px, 1fr));
    gap: 12px;
  }

  .summary-grid article,
  .panel,
  .inline-warning {
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: var(--shadow);
  }

  .summary-grid article {
    min-height: 72px;
    display: grid;
    align-content: center;
    gap: 4px;
    padding: 13px;
  }

  .summary-grid strong {
    font-size: 17px;
  }

  .summary-grid span,
  .panel-head p,
  small {
    min-width: 0;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .inline-warning {
    color: var(--warning);
    padding: 12px 14px;
  }

  .panel {
    min-width: 0;
    overflow: hidden;
  }

  .panel-head {
    min-height: 56px;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: center;
    border-bottom: 1px solid var(--line);
    background: var(--panel-alt);
    padding: 12px 14px;
  }

  .panel-head-text {
    min-width: 0;
  }

  .head-icons {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    font-size: 15px;
  }

  p {
    margin-top: 3px;
  }

  .table {
    display: grid;
    grid-template-columns:
      36px              /* GH branch    */
      minmax(0, 1fr)    /* GH name      */
      auto              /* GH meta      */
      18px              /* GH ext-link  */
      1px               /* divider      */
      36px              /* local branch */
      minmax(0, 1fr)    /* local name   */
      auto              /* badge chip   */
      auto;             /* badge label  */
    align-items: center;
    column-gap: 8px;
    max-height: min(720px, 70vh);
    overflow: auto;
    padding: 0 12px 14px;
  }

  .cell-header {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--panel-alt);
    border-bottom: 1px solid var(--line);
    min-height: 34px;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }

  .cell-header.label strong {
    color: var(--text);
    font-size: 13px;
  }

  .cell-header.divider {
    border-left: 1px solid var(--line);
    align-self: stretch;
  }

  .owner-row {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 8px 8px 0;
    color: var(--accent);
  }

  .owner-row strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cell {
    min-height: 30px;
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .cell.name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
    line-height: 30px;
  }

  .cell.divider {
    align-self: stretch;
    border-left: 1px solid var(--line);
  }

  .cell.empty {
    background: transparent;
  }

  .cell.icon {
    color: var(--muted);
  }

  a.cell {
    color: var(--text);
    text-decoration: none;
  }

  a.cell:hover {
    background: var(--hover);
  }

  a.cell.missing {
    color: var(--warning);
  }

  .cell.local-only {
    color: var(--warning);
  }

  .branch {
    color: var(--muted);
    font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
  }

  .table-empty {
    padding: 20px;
    color: var(--muted);
  }

  @media (max-width: 980px) {
    .summary-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .subnav {
      width: 100%;
    }

    .subnav button {
      flex: 1;
    }
  }
</style>
