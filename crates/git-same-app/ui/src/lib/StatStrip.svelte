<script lang="ts">
  import { currentWorkspace, selectedWorkspaceId, snapshot } from '../stores/status';
  import { summarize } from './utils';

  $: repos = $snapshot?.status?.repos ?? [];
  $: workspaceRepos = $selectedWorkspaceId
    ? repos.filter(
        (repo) =>
          repo.workspace === $selectedWorkspaceId ||
          ($currentWorkspace?.root
            ? repo.path.startsWith($currentWorkspace.root)
            : false) ||
          repo.path.startsWith($selectedWorkspaceId),
      )
    : repos;
  $: counts = summarize(workspaceRepos);
</script>

<section class="stats" aria-label="Repository status counts">
  <div><strong>{counts.total}</strong><span>Total</span></div>
  <div><strong>{counts.green}</strong><span>Synced</span></div>
  <div><strong>{counts.blue}</strong><span>Local config</span></div>
  <div><strong>{counts.orange}</strong><span>Branches</span></div>
  <div><strong>{counts.red}</strong><span>Local work</span></div>
</section>

<style>
  .stats {
    display: grid;
    grid-template-columns: repeat(5, minmax(120px, 1fr));
    gap: 12px;
    margin-bottom: 16px;
  }

  .stats div {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: var(--shadow);
    min-height: 78px;
    padding: 14px;
  }

  .stats strong {
    display: block;
    font-size: 24px;
  }

  .stats span {
    color: var(--muted);
  }

  @media (max-width: 860px) {
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
