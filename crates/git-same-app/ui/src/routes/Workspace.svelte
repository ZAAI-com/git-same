<script lang="ts">
  import { CircleDot, ExternalLink, FolderGit2, GitBranch, Settings } from 'lucide-svelte';
  import { push, router } from 'svelte-spa-router';
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
  import { formatCount, repoName } from '../lib/utils';
  import type { FinderRepoStatus, WorkspaceStructureRepoDto } from '../lib/types';

  type PairEntry = {
    remote?: WorkspaceStructureRepoDto;
    local?: FinderRepoStatus;
    name: string;
  };
  type OwnerGroup = { owner: string; entries: PairEntry[] };

  let loadedWorkspaceId = '';

  $: route = router.location || '/workspace';
  $: selectedId = $selectedWorkspaceId;
  $: if (selectedId && selectedId !== NEW_WORKSPACE_ID && selectedId !== loadedWorkspaceId) {
    loadedWorkspaceId = selectedId;
    void loadCurrentWorkspaceStructure();
  }

  $: localRepos = workspaceRepos();
  $: remoteRepos = $workspaceStructure?.repos ?? [];
  $: pairedGroups = buildPairs(remoteRepos, localRepos);
  $: allEntries = pairedGroups.flatMap((group) => group.entries);
  $: missingCount = allEntries.filter((entry) => entry.remote && !entry.local).length;
  $: localOnlyCount = allEntries.filter((entry) => !entry.remote).length;

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
          <div class="col-header-row">
            <span class="col-cell">
              <GitBranch size={15} />
              <strong>GitHub</strong>
            </span>
            <span class="col-cell col-divider" aria-hidden="true"></span>
            <span class="col-cell col-local">
              <FolderGit2 size={15} />
              <strong>Local</strong>
            </span>
          </div>

          {#each pairedGroups as group (group.owner)}
            <div class="owner-row">{group.owner}</div>

            {#each group.entries as entry (entryKey(entry))}
              <div
                class="row"
                class:missing={entry.remote && !entry.local}
                class:local-only={!entry.remote}
              >
                {#if entry.remote}
                  <a class="cell name gh-link" href={entry.remote.url} target="_blank" rel="noreferrer">
                    <span class="name-text">{entry.remote.name}</span>
                    <ExternalLink size={12} />
                  </a>
                {:else}
                  <span class="cell name placeholder">Not on GitHub</span>
                {/if}

                <span class="cell divider" aria-hidden="true"></span>

                {#if entry.local}
                  <span class="cell name">{repoName(entry.local.path)}</span>
                  <span class="cell badge"><BadgeChip badge={entry.local.badge} /></span>
                {:else}
                  <span class="cell name placeholder">Not cloned locally</span>
                  <span class="cell badge"></span>
                {/if}
              </div>
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
  .panel-head p {
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

  /* Single shared grid: every row uses subgrid so columns line up across rows. */
  .table {
    display: grid;
    grid-template-columns:
      minmax(0, 1fr)    /* GH name */
      1px               /* divider */
      minmax(0, 1fr)    /* Local name */
      110px;            /* Badge column (fixed for alignment) */
    max-height: min(720px, 70vh);
    overflow: auto;
  }

  .col-header-row,
  .row {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: subgrid;
    align-items: stretch;
    border-bottom: 1px solid var(--line);
  }

  .col-header-row {
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--panel-alt);
    height: 36px;
  }

  .col-cell {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    font-size: 13px;
    color: var(--text);
  }

  .col-cell.col-divider {
    background: var(--line);
    padding: 0;
  }

  .col-cell.col-local {
    grid-column: 3 / -1;
  }

  .owner-row {
    grid-column: 1 / -1;
    position: sticky;
    top: 36px;
    z-index: 1;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
    padding: 10px 14px 8px;
    color: var(--accent);
    font-weight: 700;
    font-size: 12px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .row {
    min-height: 40px;
    background: var(--panel);
  }

  .row:hover {
    background: var(--hover);
  }

  .cell {
    min-width: 0;
    display: flex;
    align-items: center;
  }

  .cell.name {
    overflow: hidden;
    padding: 0 14px;
    gap: 6px;
    text-decoration: none;
    color: var(--text);
  }

  .cell.name .name-text {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  a.cell.name {
    color: var(--text);
  }

  a.cell.name:hover {
    color: var(--accent);
  }

  .cell.divider {
    background: var(--line);
    padding: 0;
    align-self: stretch;
  }

  .cell.badge {
    padding: 0 14px 0 6px;
    justify-content: flex-end;
  }

  .cell.placeholder {
    color: var(--muted);
    font-style: italic;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row.missing a.cell.name {
    color: var(--warning);
  }

  .row.local-only > .cell.name:nth-child(3) {
    color: var(--warning);
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
