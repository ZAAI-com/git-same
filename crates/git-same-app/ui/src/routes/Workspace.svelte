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

  let loadedWorkspaceId = '';

  $: route = $location || '/workspace';
  $: selectedId = $selectedWorkspaceId;
  $: if (selectedId && selectedId !== NEW_WORKSPACE_ID && selectedId !== loadedWorkspaceId) {
    loadedWorkspaceId = selectedId;
    void loadCurrentWorkspaceStructure();
  }

  $: localRepos = workspaceRepos();
  $: remoteRepos = $workspaceStructure?.repos ?? [];
  $: remoteGroups = groupRemote(remoteRepos);
  $: localGroups = groupLocal(localRepos);
  $: remotePaths = new Set(remoteRepos.map((repo) => normalizePath(repo.local_path)));
  $: localOnlyCount = localRepos.filter((repo) => !remotePaths.has(normalizePath(repo.path))).length;
  $: missingCount = remoteRepos.filter((repo) => !repo.local_exists).length;

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

  function groupRemote(repos: WorkspaceStructureRepoDto[]) {
    const groups = new Map<string, WorkspaceStructureRepoDto[]>();
    for (const repo of repos) {
      const ownerRepos = groups.get(repo.owner) ?? [];
      ownerRepos.push(repo);
      groups.set(repo.owner, ownerRepos);
    }
    return [...groups.entries()].map(([owner, ownerRepos]) => ({
      owner,
      repos: ownerRepos.sort((left, right) => left.name.localeCompare(right.name)),
    }));
  }

  function groupLocal(repos: FinderRepoStatus[]) {
    const groups = new Map<string, FinderRepoStatus[]>();
    for (const repo of repos) {
      const owner = repo.org ?? ownerFromPath(repo.path);
      const ownerRepos = groups.get(owner) ?? [];
      ownerRepos.push(repo);
      groups.set(owner, ownerRepos);
    }
    return [...groups.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([owner, ownerRepos]) => ({
        owner,
        repos: ownerRepos.sort((left, right) => repoName(left.path).localeCompare(repoName(right.path))),
      }));
  }

  function ownerFromPath(path: string): string {
    const workspace = $currentWorkspace;
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

    <div class="hierarchy-grid">
      <section class="panel">
        <div class="panel-head">
          <div>
            <h2>GitHub Structure</h2>
            <p>{$workspaceStructure?.host ?? 'github.com'}</p>
          </div>
          <Github size={19} />
        </div>

        {#if $workspaceStructureLoading}
          <div class="loading">Loading GitHub structure</div>
        {:else if remoteGroups.length === 0}
          <EmptyState title="No GitHub structure" detail="Run sync or refresh discovery to populate the workspace cache." />
        {:else}
          <div class="tree">
            <div class="root-row">
              <Github size={16} />
              <strong>{$workspaceStructure?.host ?? 'github.com'}</strong>
            </div>
            {#each remoteGroups as group}
              <div class="owner-row">
                <span class="branch">├─</span>
                <strong>{group.owner}</strong>
              </div>
              {#each group.repos as repo}
                <a class:missing={!repo.local_exists} class="repo-row" href={repo.url} target="_blank" rel="noreferrer">
                  <span class="branch">│ ├─</span>
                  <span>{repo.name}</span>
                  {#if repo.local_exists}
                    <small>mirrored</small>
                  {:else}
                    <small>missing</small>
                  {/if}
                  <ExternalLink size={13} />
                </a>
              {/each}
            {/each}
          </div>
        {/if}
      </section>

      <section class="panel">
        <div class="panel-head">
          <div>
            <h2>Filesystem Structure</h2>
            <p>{$currentWorkspace.root}</p>
          </div>
          <FolderGit2 size={19} />
        </div>

        {#if localGroups.length === 0}
          <EmptyState title="No local repositories" detail="Synced repositories will appear under this workspace root." />
        {:else}
          <div class="tree">
            <div class="root-row">
              <FolderGit2 size={16} />
              <strong>{$currentWorkspace.name}</strong>
            </div>
            {#each localGroups as group}
              <div class="owner-row">
                <span class="branch">├─</span>
                <strong>{group.owner}</strong>
              </div>
              {#each group.repos as repo}
                <div class:local-only={!remotePaths.has(normalizePath(repo.path))} class="repo-row">
                  <span class="branch">│ ├─</span>
                  <span>{repoName(repo.path)}</span>
                  <BadgeChip badge={repo.badge} />
                  <small>{badgeLabel(repo.badge)}</small>
                </div>
              {/each}
            {/each}
          </div>
        {/if}
      </section>
    </div>
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

  .hierarchy-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
    align-items: start;
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

  .tree {
    max-height: min(640px, 62vh);
    overflow: auto;
    padding: 12px 10px 14px;
  }

  .root-row,
  .owner-row,
  .repo-row,
  .loading {
    min-height: 30px;
    display: grid;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
  }

  .root-row {
    grid-template-columns: 20px minmax(0, 1fr);
  }

  .owner-row {
    grid-template-columns: 34px minmax(0, 1fr);
    color: var(--accent);
  }

  .repo-row {
    grid-template-columns: 44px minmax(0, 1fr) auto auto;
    border-radius: 6px;
    color: var(--text);
    text-decoration: none;
  }

  .repo-row:hover {
    background: var(--hover);
  }

  .repo-row span,
  .owner-row strong,
  .root-row strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .branch {
    color: var(--muted);
    font-family: ui-monospace, "SFMono-Regular", Menlo, monospace;
  }

  .missing,
  .local-only {
    color: var(--warning);
  }

  .loading {
    color: var(--muted);
  }

  @media (max-width: 980px) {
    .summary-grid,
    .hierarchy-grid {
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
