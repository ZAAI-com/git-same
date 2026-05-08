<script lang="ts">
  import { Search, X } from 'lucide-svelte';
  import BadgeChip from '../lib/BadgeChip.svelte';
  import EmptyState from '../lib/EmptyState.svelte';
  import { snapshot, workspaces } from '../stores/status';
  import {
    badgeLabel,
    repoChangeCount,
    repoName,
    summarize,
  } from '../lib/utils';
  import type { Badge, FinderRepoStatus } from '../lib/types';

  let search = '';
  let badgeFilter: Badge | 'all' = 'all';
  let workspaceFilter = 'all';
  let sortBy: 'name' | 'badge' | 'changes' | 'remote' = 'badge';
  let selectedRepo: FinderRepoStatus | null = null;

  const badgeRank: Record<Badge, number> = {
    red: 0,
    orange: 1,
    blue: 2,
    gray: 3,
    green: 4,
  };

  $: repos = $snapshot?.status?.repos ?? [];
  $: counts = summarize(repos);
  $: filtered = repos
    .filter((repo) => {
      const text = `${repo.path} ${repo.org ?? ''} ${repo.workspace ?? ''} ${repo.current_branch}`.toLowerCase();
      const matchesSearch = !search.trim() || text.includes(search.trim().toLowerCase());
      const matchesBadge = badgeFilter === 'all' || repo.badge === badgeFilter;
      const matchesWorkspace =
        workspaceFilter === 'all' ||
        repo.workspace === workspaceFilter ||
        $workspaces.some(
          (workspace) => workspace.id === workspaceFilter && repo.path.startsWith(workspace.root),
        );
      return matchesSearch && matchesBadge && matchesWorkspace;
    })
    .sort((left, right) => {
      if (sortBy === 'name') return repoName(left.path).localeCompare(repoName(right.path));
      if (sortBy === 'changes') return repoChangeCount(right) - repoChangeCount(left);
      if (sortBy === 'remote') return right.ahead + right.behind - (left.ahead + left.behind);
      return badgeRank[left.badge] - badgeRank[right.badge];
    });
</script>

<section class="badge-browser">
  <section class="toolbar">
    <label class="search">
      <Search size={16} />
      <input bind:value={search} placeholder="Search repositories, orgs, branches" />
    </label>
    <select bind:value={badgeFilter} aria-label="Badge filter">
      <option value="all">All badges</option>
      <option value="green">Synced</option>
      <option value="blue">Local config</option>
      <option value="orange">Branches</option>
      <option value="red">Local work</option>
      <option value="gray">Pending</option>
    </select>
    <select bind:value={workspaceFilter} aria-label="Workspace filter">
      <option value="all">All workspaces</option>
      {#each $workspaces as workspace}
        <option value={workspace.id}>{workspace.name}</option>
      {/each}
    </select>
    <select bind:value={sortBy} aria-label="Sort repositories">
      <option value="badge">Sort by urgency</option>
      <option value="name">Sort by name</option>
      <option value="changes">Sort by changes</option>
      <option value="remote">Sort by remote drift</option>
    </select>
  </section>

  <div class="badge-summary">
    <BadgeChip badge="green" count={counts.green} />
    <BadgeChip badge="blue" count={counts.blue} />
    <BadgeChip badge="orange" count={counts.orange} />
    <BadgeChip badge="red" count={counts.red} />
    <BadgeChip badge="gray" count={counts.gray} />
  </div>

  <section class="browser-grid" class:has-detail={Boolean(selectedRepo)}>
    <section class="table-panel">
      <div class="table-head">
        <span>Repository</span>
        <span>Badge</span>
        <span>Branch</span>
        <span>Changes</span>
        <span>Remote</span>
      </div>
      {#if filtered.length === 0}
        <EmptyState title="No repositories match" detail="Adjust the badge, workspace, or search filters." />
      {:else}
        {#each filtered as repo}
          <button
            type="button"
            class:selected={selectedRepo?.path === repo.path}
            class="repo-row"
            on:click={() => (selectedRepo = repo)}
          >
            <span class="repo-name">
              <strong>{repoName(repo.path)}</strong>
              <small>{repo.org ?? repo.workspace ?? repo.path}</small>
            </span>
            <BadgeChip badge={repo.badge} />
            <span>{repo.current_branch}</span>
            <span>{repoChangeCount(repo)}</span>
            <span>{repo.ahead} ahead / {repo.behind} behind</span>
          </button>
        {/each}
      {/if}
    </section>

    {#if selectedRepo}
      <aside class="detail">
        <header>
          <div>
            <h2>{repoName(selectedRepo.path)}</h2>
            <p>{selectedRepo.path}</p>
          </div>
          <button type="button" aria-label="Close details" on:click={() => (selectedRepo = null)}>
            <X size={16} />
          </button>
        </header>

        <BadgeChip badge={selectedRepo.badge} />

        <dl>
          <div><dt>Badge</dt><dd>{badgeLabel(selectedRepo.badge)}</dd></div>
          <div><dt>Branch</dt><dd>{selectedRepo.current_branch}</dd></div>
          <div><dt>Default branch</dt><dd>{selectedRepo.default_branch ?? 'Unknown'}</dd></div>
          <div><dt>Changes</dt><dd>{repoChangeCount(selectedRepo)}</dd></div>
          <div><dt>Remote</dt><dd>{selectedRepo.ahead} ahead / {selectedRepo.behind} behind</dd></div>
          <div><dt>Stash</dt><dd>{selectedRepo.stash_count}</dd></div>
        </dl>

        {#if selectedRepo.read_error}
          <section class="detail-section error">
            <h3>Read Error</h3>
            <p>{selectedRepo.read_error}</p>
          </section>
        {/if}

        <section class="detail-section">
          <h3>Branches</h3>
          {#if selectedRepo.branches.length === 0}
            <p>No branch details recorded.</p>
          {:else}
            {#each selectedRepo.branches as branch}
              <article>
                <strong>{branch.name}</strong>
                <small>{branch.upstream ?? 'No upstream'} · {branch.ahead} ahead / {branch.behind} behind</small>
              </article>
            {/each}
          {/if}
        </section>

        <section class="detail-section">
          <h3>Remotes</h3>
          {#if selectedRepo.remotes.length === 0}
            <p>No remotes recorded.</p>
          {:else}
            {#each selectedRepo.remotes as remote}
              <article>
                <strong>{remote.name}</strong>
                <small>{remote.url}</small>
              </article>
            {/each}
          {/if}
        </section>

        <section class="detail-section">
          <h3>Worktrees</h3>
          {#if selectedRepo.worktrees.length === 0}
            <p>No extra worktrees recorded.</p>
          {:else}
            {#each selectedRepo.worktrees as worktree}
              <article>
                <strong>{worktree.branch ?? 'Detached'}</strong>
                <small>{worktree.path} · {worktree.synced ? 'synced' : 'not synced'}</small>
              </article>
            {/each}
          {/if}
        </section>

        {#if selectedRepo.important_ignored_files?.length}
          <section class="detail-section">
            <h3>Important Ignored Files</h3>
            {#each selectedRepo.important_ignored_files as file}
              <code>{file}</code>
            {/each}
          </section>
        {/if}
      </aside>
    {/if}
  </section>
</section>

<style>
  .badge-browser {
    display: grid;
    gap: 14px;
  }

  .toolbar,
  .badge-summary,
  .table-panel,
  .detail {
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: var(--shadow);
  }

  .toolbar {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) repeat(3, minmax(150px, 0.35fr));
    gap: 10px;
    padding: 12px;
  }

  .search {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-alt);
    padding: 0 10px;
  }

  input,
  select {
    width: 100%;
    height: 34px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-alt);
    color: var(--text);
    padding: 0 10px;
    font: inherit;
  }

  .search input {
    border: 0;
    background: transparent;
    padding: 0;
  }

  .badge-summary {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    padding: 12px;
  }

  .browser-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 14px;
  }

  .browser-grid.has-detail {
    grid-template-columns: minmax(0, 1fr) minmax(300px, 360px);
  }

  .table-panel {
    overflow: hidden;
  }

  .table-head,
  .repo-row {
    display: grid;
    grid-template-columns: minmax(200px, 1.8fr) 118px 130px 86px 140px;
    align-items: center;
    gap: 12px;
    min-height: 46px;
    padding: 0 14px;
  }

  .table-head {
    background: var(--panel-alt);
    color: var(--muted);
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
  }

  .repo-row {
    width: 100%;
    border: 0;
    border-top: 1px solid var(--line);
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    text-align: left;
  }

  .repo-row:hover,
  .repo-row.selected {
    background: var(--hover);
  }

  .repo-name {
    min-width: 0;
  }

  .repo-name strong,
  .repo-name small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  small,
  p,
  dt,
  .repo-name small {
    color: var(--muted);
  }

  .detail {
    align-self: start;
    display: grid;
    gap: 14px;
    padding: 16px;
  }

  .detail header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 10px;
  }

  .detail h2,
  .detail h3,
  .detail p {
    margin: 0;
  }

  .detail h2 {
    font-size: 17px;
  }

  .detail header p {
    margin-top: 4px;
    overflow-wrap: anywhere;
  }

  .detail header button {
    width: 30px;
    height: 30px;
    display: inline-grid;
    place-items: center;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-alt);
    color: var(--text);
    cursor: pointer;
  }

  dl {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin: 0;
  }

  dt {
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
  }

  dd {
    margin: 3px 0 0;
    overflow-wrap: anywhere;
  }

  .detail-section {
    display: grid;
    gap: 8px;
    border-top: 1px solid var(--line);
    padding-top: 12px;
  }

  .detail-section h3 {
    font-size: 13px;
  }

  .detail-section article {
    display: grid;
    gap: 2px;
  }

  .detail-section small {
    overflow-wrap: anywhere;
  }

  .detail-section.error {
    color: var(--danger);
  }

  code {
    width: fit-content;
    max-width: 100%;
    overflow-wrap: anywhere;
    border-radius: 6px;
    background: var(--panel-alt);
    padding: 4px 6px;
  }

  @media (max-width: 1180px) {
    .toolbar,
    .browser-grid.has-detail {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 860px) {
    .table-head {
      display: none;
    }

    .repo-row {
      grid-template-columns: 1fr;
      justify-items: start;
      gap: 6px;
      padding: 12px 14px;
    }
  }
</style>
