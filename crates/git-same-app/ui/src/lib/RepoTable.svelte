<script lang="ts">
  import { selectedWorkspaceId, snapshot } from '../stores/status';
  import { badgeLabel, repoName } from './utils';

  $: repos = $snapshot?.status?.repos ?? [];
  $: workspaceRepos = $selectedWorkspaceId
    ? repos.filter(
        (repo) =>
          repo.workspace === $selectedWorkspaceId ||
          repo.path.startsWith($selectedWorkspaceId),
      )
    : repos;
</script>

<section class="repo-table">
  <div class="table-head">
    <span>Repository</span>
    <span>State</span>
    <span>Branch</span>
    <span>Changes</span>
    <span>Remote</span>
  </div>
  {#if workspaceRepos.length === 0}
    <div class="empty">No status rows</div>
  {:else}
    {#each workspaceRepos.slice(0, 200) as repo}
      <article class="repo-row">
        <div>
          <strong>{repoName(repo.path)}</strong>
          <small>{repo.org ?? repo.workspace ?? repo.path}</small>
        </div>
        <span class={`badge ${repo.badge}`}>{badgeLabel(repo.badge)}</span>
        <span>{repo.current_branch}</span>
        <span
          >{repo.staged_count + repo.unstaged_count + repo.untracked_count}</span
        >
        <span>{repo.ahead} ahead / {repo.behind} behind</span>
      </article>
    {/each}
  {/if}
</section>

<style>
  .repo-table {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: var(--shadow);
    overflow: hidden;
  }

  .table-head,
  .repo-row {
    display: grid;
    grid-template-columns: minmax(180px, 1.7fr) 120px 130px 90px 140px;
    gap: 12px;
    align-items: center;
    min-height: 48px;
    padding: 0 14px;
  }

  .table-head {
    background: var(--panel-alt);
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .repo-row {
    border-top: 1px solid var(--line);
  }

  .repo-row strong,
  .repo-row small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .repo-row small {
    color: var(--muted);
  }

  .badge {
    width: fit-content;
    min-width: 84px;
    border-radius: 999px;
    padding: 4px 8px;
    font-size: 12px;
    font-weight: 700;
    text-align: center;
  }

  .green {
    background: color-mix(in srgb, var(--ok) 16%, transparent);
    color: var(--ok);
  }

  .blue {
    background: color-mix(in srgb, var(--blue) 16%, transparent);
    color: var(--blue);
  }

  .orange {
    background: color-mix(in srgb, var(--warning) 18%, transparent);
    color: var(--warning);
  }

  .red {
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    color: var(--danger);
  }

  .gray {
    background: var(--panel-alt);
    color: var(--muted);
  }

  .empty {
    padding: 18px;
  }

  @media (max-width: 860px) {
    .table-head {
      display: none;
    }

    .repo-row {
      grid-template-columns: 1fr;
      gap: 6px;
      padding: 12px 14px;
    }
  }
</style>
