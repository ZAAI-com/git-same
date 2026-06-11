<script lang="ts">
  import {
    ArrowLeft,
    ArrowRight,
    CircleDot,
    ExternalLink,
    FolderGit2,
    GitBranch,
    Link,
    Search,
    Settings,
  } from 'lucide-svelte';
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
  import type {
    FinderRepoStatus,
    WorkspaceStructureDto,
    WorkspaceStructureRepoDto,
  } from '../lib/types';

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
  // Reactive so the subtitle re-renders when the structure loads (a bare
  // `{sourceLabel()}` call would only run once at init and never update).
  $: sourceText = sourceLabel($workspaceStructure);

  let filter = '';
  $: visibleGroups = applyFilter(pairedGroups, filter);
  $: matchedCount = allEntries.length - missingCount - localOnlyCount;

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

  function entryStatus(entry: PairEntry): 'matched' | 'missing' | 'local-only' {
    if (!entry.remote) return 'local-only';
    if (!entry.local) return 'missing';
    return 'matched';
  }

  function applyFilter(groups: OwnerGroup[], term: string): OwnerGroup[] {
    const needle = term.trim().toLowerCase();
    if (!needle) return groups;
    return groups
      .map((group) => ({
        owner: group.owner,
        entries: group.entries.filter((entry) => entryMatches(entry, needle)),
      }))
      .filter((group) => group.entries.length > 0);
  }

  function entryMatches(entry: PairEntry, needle: string): boolean {
    if (entry.name.toLowerCase().includes(needle)) return true;
    if (entry.remote?.full_name.toLowerCase().includes(needle)) return true;
    if (entry.local && repoName(entry.local.path).toLowerCase().includes(needle)) return true;
    return false;
  }

  function sourceLabel(structure: WorkspaceStructureDto | null): string {
    if (!structure) return 'No structure loaded';
    if (structure.source === 'remote') return 'GitHub structure · live';
    if (structure.source === 'cache') {
      const age = cacheAgeLabel(structure.cache_age_secs);
      return age ? `GitHub structure · cached ${age}` : 'GitHub structure · cached';
    }
    // source === 'unavailable': repos may still be served from cache.
    if (structure.repos.length > 0) return 'GitHub structure · cached (offline)';
    return 'GitHub structure unavailable';
  }

  function cacheAgeLabel(secs: number | null): string {
    if (secs == null || secs < 0) return '';
    if (secs < 60) return 'just now';
    const minutes = Math.floor(secs / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    return `${Math.floor(hours / 24)}d ago`;
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
        <span>{sourceText}</span>
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
          <p>
            {$workspaceStructure?.host ?? 'github.com'} · {matchedCount} paired · {$currentWorkspace.root}
          </p>
        </div>
        <label class="search" aria-label="Filter repositories">
          <Search size={15} />
          <input type="text" placeholder="Filter repos…" bind:value={filter} />
        </label>
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
            <span class="col-cell col-status" aria-hidden="true"></span>
            <span class="col-cell col-local">
              <FolderGit2 size={15} />
              <strong>Local</strong>
            </span>
          </div>

          {#if visibleGroups.length === 0}
            <div class="table-note">No repositories match “{filter}”.</div>
          {:else}
            {#each visibleGroups as group (group.owner)}
              <div class="owner-row">
                <span>{group.owner}</span>
                <small>{formatCount(group.entries.length, 'repo')}</small>
              </div>

              {#each group.entries as entry, i (entryKey(entry))}
                {@const status = entryStatus(entry)}
                <div
                  class="row"
                  class:alt={i % 2 === 1}
                  class:missing={status === 'missing'}
                  class:local-only={status === 'local-only'}
                >
                  {#if entry.remote}
                    <a class="cell name gh-link" href={entry.remote.url} target="_blank" rel="noreferrer">
                      <span class="name-text">{entry.remote.name}</span>
                      <ExternalLink class="gh-ext" size={12} />
                    </a>
                  {:else}
                    <span class="cell name ghost">not on GitHub</span>
                  {/if}

                  <span class="cell status" aria-hidden="true">
                    {#if status === 'matched'}
                      <Link size={12} />
                    {:else if status === 'missing'}
                      <ArrowRight size={14} />
                    {:else}
                      <ArrowLeft size={14} />
                    {/if}
                  </span>

                  {#if entry.local}
                    <span class="cell name">{repoName(entry.local.path)}</span>
                    <span class="cell badge"><BadgeChip badge={entry.local.badge} /></span>
                  {:else}
                    <span class="cell name ghost">missing locally</span>
                    <span class="cell badge"></span>
                  {/if}
                </div>
              {/each}
            {/each}
          {/if}
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
    flex: 1 1 auto;
    min-width: 0;
  }

  .search {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel);
    color: var(--muted);
  }

  .search input {
    width: 170px;
    max-width: 40vw;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
  }

  .search input::placeholder {
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

  /* Single shared grid: every row uses subgrid so columns line up across rows. */
  .table {
    display: grid;
    grid-template-columns:
      minmax(0, 1fr)    /* GH name */
      44px              /* status gutter (seam between the two panels) */
      minmax(0, 1fr)    /* Local name */
      104px;            /* Badge column (fixed for alignment) */
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

  .col-cell.col-status {
    padding: 0;
    border-left: 1px solid var(--line);
    border-right: 1px solid var(--line);
  }

  .col-cell.col-local {
    grid-column: 3 / -1;
  }

  .owner-row {
    grid-column: 1 / -1;
    position: sticky;
    top: 36px;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
    border-left: 3px solid var(--accent);
    padding: 7px 14px 6px 11px;
    color: var(--accent);
    font-weight: 700;
    font-size: 12px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .owner-row small {
    color: var(--muted);
    font-weight: 600;
    letter-spacing: 0;
    text-transform: none;
  }

  .row {
    min-height: 32px;
    background: var(--panel);
  }

  .row.alt {
    background: var(--panel-alt);
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

  .cell.status {
    justify-content: center;
    align-self: stretch;
    color: var(--muted);
    border-left: 1px solid var(--line);
    border-right: 1px solid var(--line);
  }

  .row.missing .cell.status,
  .row.local-only .cell.status {
    color: var(--warning);
  }

  .cell.badge {
    padding: 0 14px 0 6px;
    justify-content: flex-end;
  }

  .cell.ghost {
    color: var(--muted);
    opacity: 0.6;
    font-style: italic;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* External-link glyph stays hidden until the row is hovered or focused. */
  .gh-link :global(.gh-ext) {
    opacity: 0;
    transition: opacity 0.12s ease;
  }

  .row:hover .gh-link :global(.gh-ext),
  .gh-link:focus-visible :global(.gh-ext) {
    opacity: 0.7;
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

  .table-note {
    grid-column: 1 / -1;
    padding: 18px 14px;
    color: var(--muted);
    text-align: center;
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
