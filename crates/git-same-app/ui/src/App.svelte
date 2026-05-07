<script lang="ts">
  import { onMount } from 'svelte';
  import { AlertTriangle, FolderGit2, RefreshCw, Settings, ShieldCheck } from 'lucide-svelte';
  import { listWorkspaces, onStatusUpdated, readStatus, startSync } from './lib/tauri';
  import type { Badge, FinderRepoStatus, StatusSnapshot, WorkspaceSummary } from './lib/types';

  let workspaces: WorkspaceSummary[] = [];
  let snapshot: StatusSnapshot | null = null;
  let selectedWorkspace = '';
  let loading = true;
  let syncing = '';
  let error = '';
  let activeView: 'dashboard' | 'settings' = 'dashboard';

  $: repos = snapshot?.status?.repos ?? [];
  $: workspaceRepos = selectedWorkspace
    ? repos.filter((repo) => repo.workspace === selectedWorkspace || repo.path.startsWith(selectedWorkspace))
    : repos;
  $: counts = summarize(repos);
  $: currentWorkspace =
    workspaces.find((workspace) => workspace.id === selectedWorkspace) ?? workspaces[0];

  onMount(() => {
    let unsubscribe: (() => void) | undefined;

    void (async () => {
      try {
        await refresh();
        unsubscribe = await onStatusUpdated((next) => {
          snapshot = next;
        });
      } catch (err) {
        error = String(err);
      } finally {
        loading = false;
      }
    })();

    return () => {
      unsubscribe?.();
    };
  });

  async function refresh() {
    error = '';
    const [workspaceList, status] = await Promise.all([listWorkspaces(), readStatus()]);
    workspaces = workspaceList;
    snapshot = status;
    if (!selectedWorkspace && workspaces.length > 0) {
      selectedWorkspace = workspaces.find((workspace) => workspace.default)?.id ?? workspaces[0].id;
    }
  }

  async function syncCurrent() {
    if (!currentWorkspace) return;
    syncing = currentWorkspace.id;
    error = '';
    try {
      await startSync(currentWorkspace.id);
      await refresh();
    } catch (err) {
      error = String(err);
    } finally {
      syncing = '';
    }
  }

  function summarize(items: FinderRepoStatus[]) {
    return items.reduce(
      (acc, repo) => {
        acc.total += 1;
        acc[repo.badge] += 1;
        return acc;
      },
      { total: 0, green: 0, blue: 0, orange: 0, red: 0, gray: 0 } as Record<Badge | 'total', number>,
    );
  }

  function badgeLabel(badge: Badge) {
    return {
      green: 'Synced',
      blue: 'Local config',
      orange: 'Branches',
      red: 'Local work',
      gray: 'Pending',
    }[badge];
  }

  function repoName(path: string) {
    return path.split('/').filter(Boolean).at(-1) ?? path;
  }

  function relativeTime(value: string | null | undefined) {
    if (!value) return 'Never';
    const timestamp = Date.parse(value);
    if (Number.isNaN(timestamp)) return value;
    const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 48) return `${hours}h ago`;
    return new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(timestamp);
  }
</script>

<svelte:head>
  <title>git-Same</title>
</svelte:head>

<main class="shell">
  <aside class="sidebar">
    <div class="brand">
      <FolderGit2 size={22} />
      <span>git-Same</span>
    </div>

    <nav class="nav">
      <button class:active={activeView === 'dashboard'} on:click={() => (activeView = 'dashboard')}>
        <ShieldCheck size={17} />
        <span>Dashboard</span>
      </button>
      <button class:active={activeView === 'settings'} on:click={() => (activeView = 'settings')}>
        <Settings size={17} />
        <span>Settings</span>
      </button>
    </nav>

    <div class="workspace-list">
      {#each workspaces as workspace}
        <button
          class:active={selectedWorkspace === workspace.id}
          on:click={() => (selectedWorkspace = workspace.id)}
          title={workspace.root}
        >
          <span>{workspace.id}</span>
          <small>{workspace.provider}</small>
        </button>
      {/each}
    </div>
  </aside>

  <section class="content">
    <header class="topbar">
      <div>
        <h1>{activeView === 'settings' ? 'Settings' : currentWorkspace?.id ?? 'Dashboard'}</h1>
        <p>{currentWorkspace?.root ?? snapshot?.status_path ?? 'No workspace selected'}</p>
      </div>
      <div class="actions">
        <button class="icon-button" on:click={refresh} title="Refresh" aria-label="Refresh">
          <RefreshCw size={18} class={loading ? 'spinning' : ''} />
        </button>
        {#if activeView === 'dashboard'}
          <button class="primary" on:click={syncCurrent} disabled={!currentWorkspace || Boolean(syncing)}>
            <RefreshCw size={17} class={syncing ? 'spinning' : ''} />
            <span>{syncing ? 'Syncing' : 'Sync'}</span>
          </button>
        {/if}
      </div>
    </header>

    {#if error}
      <div class="banner error">
        <AlertTriangle size={18} />
        <span>{error}</span>
      </div>
    {:else if snapshot?.stale}
      <div class="banner warning">
        <AlertTriangle size={18} />
        <span>Daemon stopped</span>
        <code>launchctl load ~/Library/LaunchAgents/com.zaai.git-same.daemon.plist</code>
      </div>
    {/if}

    {#if activeView === 'settings'}
      <section class="settings-panel">
        <h2>Finder Badges</h2>
        <dl>
          <div>
            <dt>Status file</dt>
            <dd>{snapshot?.status_path ?? 'Unavailable'}</dd>
          </div>
          <div>
            <dt>Last update</dt>
            <dd>{relativeTime(snapshot?.updated_at)}</dd>
          </div>
          <div>
            <dt>Daemon PID</dt>
            <dd>{snapshot?.status?.daemon_pid ?? 'Unavailable'}</dd>
          </div>
        </dl>
      </section>
    {:else}
      <section class="stats" aria-label="Repository status counts">
        <div><strong>{counts.total}</strong><span>Total</span></div>
        <div><strong>{counts.green}</strong><span>Synced</span></div>
        <div><strong>{counts.blue}</strong><span>Local config</span></div>
        <div><strong>{counts.orange}</strong><span>Branches</span></div>
        <div><strong>{counts.red}</strong><span>Local work</span></div>
      </section>

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
              <span>{repo.staged_count + repo.unstaged_count + repo.untracked_count}</span>
              <span>{repo.ahead} ahead / {repo.behind} behind</span>
            </article>
          {/each}
        {/if}
      </section>
    {/if}
  </section>
</main>

<style>
  .shell {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
    min-height: 100vh;
    color: var(--text);
  }

  .sidebar {
    border-right: 1px solid var(--line);
    background: var(--panel);
    padding: 18px 14px;
  }

  .brand,
  .nav button,
  .actions,
  .banner,
  .primary,
  .workspace-list button {
    display: flex;
    align-items: center;
  }

  .brand {
    gap: 10px;
    height: 40px;
    font-size: 18px;
    font-weight: 700;
  }

  .nav {
    display: grid;
    gap: 6px;
    margin: 24px 0;
  }

  .nav button,
  .workspace-list button {
    width: 100%;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    text-align: left;
  }

  .nav button {
    gap: 9px;
    height: 38px;
    padding: 0 10px;
  }

  .nav button.active,
  .workspace-list button.active {
    background: var(--panel-alt);
  }

  .workspace-list {
    display: grid;
    gap: 8px;
  }

  .workspace-list button {
    display: grid;
    gap: 3px;
    min-height: 58px;
    padding: 10px;
  }

  .workspace-list span,
  .workspace-list small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .workspace-list small,
  .topbar p,
  .repo-row small {
    color: var(--muted);
  }

  .content {
    min-width: 0;
    padding: 22px;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    gap: 18px;
    align-items: center;
    margin-bottom: 18px;
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    font-size: 24px;
    line-height: 1.2;
  }

  .topbar p {
    margin-top: 5px;
    overflow-wrap: anywhere;
  }

  .actions {
    gap: 8px;
  }

  .icon-button,
  .primary {
    border: 1px solid var(--line);
    border-radius: 8px;
    cursor: pointer;
  }

  .icon-button {
    width: 38px;
    height: 38px;
    display: inline-grid;
    place-items: center;
    background: var(--panel);
    color: var(--text);
  }

  .primary {
    gap: 8px;
    height: 38px;
    padding: 0 13px;
    background: var(--accent);
    color: white;
    font-weight: 700;
  }

  .primary:disabled {
    cursor: not-allowed;
    opacity: 0.65;
  }

  .banner {
    gap: 10px;
    min-height: 42px;
    margin-bottom: 16px;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: var(--panel);
    overflow-wrap: anywhere;
  }

  .banner.warning {
    color: var(--warning);
  }

  .banner.error {
    color: var(--danger);
  }

  code {
    color: var(--text);
    background: var(--panel-alt);
    padding: 3px 6px;
    border-radius: 6px;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(5, minmax(120px, 1fr));
    gap: 12px;
    margin-bottom: 16px;
  }

  .stats div,
  .repo-table,
  .settings-panel {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: var(--shadow);
  }

  .stats div {
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

  .repo-table {
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

  .empty,
  .settings-panel {
    padding: 18px;
  }

  .settings-panel dl {
    display: grid;
    gap: 12px;
    margin: 16px 0 0;
  }

  .settings-panel div {
    display: grid;
    gap: 4px;
  }

  .settings-panel dt {
    color: var(--muted);
    font-size: 12px;
    text-transform: uppercase;
  }

  .settings-panel dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 860px) {
    .shell {
      grid-template-columns: 1fr;
    }

    .sidebar {
      border-right: 0;
      border-bottom: 1px solid var(--line);
    }

    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

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
