<script lang="ts">
  import { onMount } from 'svelte';
  import { CircleDot, FolderOpen, Save, Search, Settings, Trash2 } from '@lucide/svelte';
  import { push, router } from 'svelte-spa-router';
  import EmptyState from '../lib/EmptyState.svelte';
  import {
    NEW_WORKSPACE_ID,
    currentWorkspace,
    errorMessage,
    removeWorkspace,
    saveWorkspace,
    selectedWorkspaceId,
    workspaces,
  } from '../stores/status';
  import { chooseFolder, discoverProviderOrgs, readWorkspace } from '../lib/tauri';
  import { linesToList, listToLines, relativeTime } from '../lib/utils';
  import type {
    CloneOptionsDto,
    FilterOptionsDto,
    SyncMode,
    WorkspaceDetailDto,
    WorkspaceInput,
    WorkspaceProviderDto,
  } from '../lib/types';

  let loadedId = '';
  let loading = false;
  let saving = false;
  let discovering = false;
  let workspaceId: string | null = null;
  let root = '';
  let providerKind = 'github';
  let preferSsh = true;
  let apiUrl = '';
  let username = '';
  let orgsText = '';
  let includeReposText = '';
  let excludeReposText = '';
  let structure = '';
  let syncMode: SyncMode | '' = '';
  let useCloneOverride = false;
  let cloneDepth = 0;
  let cloneBranch = '';
  let cloneSubmodules = false;
  let includeArchived = false;
  let includeForks = false;
  let filterOrgsText = '';
  let filterExcludeText = '';
  let concurrencyText = '';
  let refreshIntervalText = '';
  let isDefault = false;
  let lastSynced: string | null = null;
  let configPath = '';

  $: route = router.location || '/workspace/screen';
  $: selectedId = $selectedWorkspaceId;
  $: if (selectedId !== loadedId) {
    loadedId = selectedId;
    void loadSelected(selectedId);
  }

  onMount(() => {
    if (!selectedId && $workspaces.length === 0) {
      selectedWorkspaceId.set(NEW_WORKSPACE_ID);
    }
  });

  function emptyClone(): CloneOptionsDto {
    return { depth: 0, branch: '', recurse_submodules: false };
  }

  function emptyFilters(): FilterOptionsDto {
    return {
      include_archived: false,
      include_forks: false,
      orgs: [],
      exclude_repos: [],
    };
  }

  function providerInput(): WorkspaceProviderDto {
    return {
      kind: providerKind,
      label: providerKind === 'github' ? 'GitHub' : providerKind,
      api_url: apiUrl.trim() || null,
      prefer_ssh: preferSsh,
    };
  }

  async function loadSelected(id: string) {
    const targetId = id || $currentWorkspace?.id || NEW_WORKSPACE_ID;
    if (targetId === NEW_WORKSPACE_ID) {
      applyNewWorkspace();
      return;
    }

    loading = true;
    try {
      applyWorkspace(await readWorkspace(targetId));
    } catch (err) {
      errorMessage.set(String(err));
      applyNewWorkspace();
    } finally {
      loading = false;
    }
  }

  function applyNewWorkspace() {
    workspaceId = null;
    root = '';
    providerKind = 'github';
    preferSsh = true;
    apiUrl = '';
    username = '';
    orgsText = '';
    includeReposText = '';
    excludeReposText = '';
    structure = '';
    syncMode = '';
    useCloneOverride = false;
    cloneDepth = 0;
    cloneBranch = '';
    cloneSubmodules = false;
    includeArchived = false;
    includeForks = false;
    filterOrgsText = '';
    filterExcludeText = '';
    concurrencyText = '';
    refreshIntervalText = '';
    isDefault = $workspaces.length === 0;
    lastSynced = null;
    configPath = '';
  }

  function applyWorkspace(detail: WorkspaceDetailDto) {
    workspaceId = detail.id;
    root = detail.root;
    providerKind = detail.provider.kind;
    preferSsh = detail.provider.prefer_ssh;
    apiUrl = detail.provider.api_url ?? '';
    username = detail.username;
    orgsText = listToLines(detail.orgs);
    includeReposText = listToLines(detail.include_repos);
    excludeReposText = listToLines(detail.exclude_repos);
    structure = detail.structure ?? '';
    syncMode = detail.sync_mode ?? '';
    useCloneOverride = Boolean(detail.clone_options);
    const clone = detail.clone_options ?? emptyClone();
    cloneDepth = clone.depth;
    cloneBranch = clone.branch;
    cloneSubmodules = clone.recurse_submodules;
    includeArchived = detail.filters.include_archived;
    includeForks = detail.filters.include_forks;
    filterOrgsText = listToLines(detail.filters.orgs);
    filterExcludeText = listToLines(detail.filters.exclude_repos);
    concurrencyText = detail.concurrency?.toString() ?? '';
    refreshIntervalText = detail.refresh_interval?.toString() ?? '';
    isDefault = detail.default;
    lastSynced = detail.last_synced;
    configPath = detail.config_path;
  }

  async function browseRoot() {
    const selected = await chooseFolder(root || undefined);
    if (selected) root = selected;
  }

  async function discoverOrgs() {
    discovering = true;
    try {
      const discovery = await discoverProviderOrgs(providerInput());
      if (discovery.username) username = discovery.username;
      orgsText = discovery.orgs
        .filter((org) => org.selected)
        .map((org) => org.name)
        .join('\n');
    } catch (err) {
      errorMessage.set(String(err));
    } finally {
      discovering = false;
    }
  }

  async function submit() {
    saving = true;
    try {
      const filters: FilterOptionsDto = {
        ...emptyFilters(),
        include_archived: includeArchived,
        include_forks: includeForks,
        orgs: linesToList(filterOrgsText),
        exclude_repos: linesToList(filterExcludeText),
      };
      const cloneOptions: CloneOptionsDto | null = useCloneOverride
        ? {
            depth: Number(cloneDepth) || 0,
            branch: cloneBranch.trim(),
            recurse_submodules: cloneSubmodules,
          }
        : null;
      const input: WorkspaceInput = {
        id: workspaceId,
        root,
        provider: providerInput(),
        username,
        orgs: linesToList(orgsText),
        include_repos: linesToList(includeReposText),
        exclude_repos: linesToList(excludeReposText),
        structure: structure.trim() || null,
        sync_mode: syncMode || null,
        clone_options: cloneOptions,
        filters,
        concurrency: optionalNumber(concurrencyText),
        refresh_interval: optionalNumber(refreshIntervalText),
        default: isDefault,
      };
      const savedId = await saveWorkspace(input);
      loadedId = '';
      await loadSelected(savedId);
    } catch (err) {
      errorMessage.set(String(err));
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!workspaceId) return;
    if (!window.confirm('Remove this workspace from Git-Same? Repository folders stay on disk.')) {
      return;
    }
    await removeWorkspace(workspaceId);
    loadedId = '';
  }

  function optionalNumber(value: string): number | null {
    const trimmed = value.trim();
    if (!trimmed) return null;
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) ? parsed : null;
  }
</script>

<section class="workspace-screen">
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

  {#if loading}
    <EmptyState title="Loading workspace" detail="Reading the portable workspace config." />
  {:else}
  <form class="workspace-form" on:submit|preventDefault={submit}>
    <section class="panel hero">
      <div>
        <h2>{workspaceId ? 'Workspace Details' : 'Create Workspace'}</h2>
        <p>{workspaceId ? configPath : 'A new .git-same/config.toml will be created inside the selected folder.'}</p>
      </div>
      <div class="actions">
        {#if workspaceId}
          <button class="danger" type="button" on:click={remove}>
            <Trash2 size={16} />
            <span>Delete</span>
          </button>
        {/if}
        <button class="primary" type="submit" disabled={saving}>
          <Save size={16} />
          <span>{saving ? 'Saving' : 'Save'}</span>
        </button>
      </div>
    </section>

    <section class="panel fields">
      <h2>Location</h2>
      <label class="wide">
        <span>Root folder</span>
        <div class="input-action">
          <input bind:value={root} required placeholder="/Users/me/GitHub" />
          <button type="button" on:click={browseRoot}>
            <FolderOpen size={16} />
            <span>Choose</span>
          </button>
        </div>
      </label>
      <label>
        <span>Default workspace</span>
        <input class="check" type="checkbox" bind:checked={isDefault} />
      </label>
      <label>
        <span>Last synced</span>
        <input value={relativeTime(lastSynced)} disabled />
      </label>
    </section>

    <section class="panel fields">
      <h2>Provider</h2>
      <label>
        <span>Provider</span>
        <select bind:value={providerKind}>
          <option value="github">GitHub</option>
          <option value="github-enterprise" disabled>GitHub Enterprise</option>
          <option value="gitlab" disabled>GitLab</option>
          <option value="codeberg" disabled>Codeberg</option>
          <option value="bitbucket" disabled>Bitbucket</option>
        </select>
      </label>
      <label>
        <span>Username</span>
        <input bind:value={username} placeholder="Detected by gh auth" />
      </label>
      <label>
        <span>Prefer SSH clone URLs</span>
        <input class="check" type="checkbox" bind:checked={preferSsh} />
      </label>
      <label>
        <span>API URL override</span>
        <input bind:value={apiUrl} placeholder="Optional" />
      </label>
      <div class="wide">
        <button class="secondary" type="button" on:click={discoverOrgs} disabled={discovering}>
          <Search size={16} class={discovering ? 'spinning' : ''} />
          <span>{discovering ? 'Discovering' : 'Discover organizations'}</span>
        </button>
      </div>
    </section>

    <section class="panel fields">
      <h2>Repository Selection</h2>
      <label>
        <span>Organizations</span>
        <textarea bind:value={orgsText} placeholder="One org per line; empty means all"></textarea>
      </label>
      <label>
        <span>Include repos</span>
        <textarea bind:value={includeReposText} placeholder="org/repo"></textarea>
      </label>
      <label>
        <span>Exclude repos</span>
        <textarea bind:value={excludeReposText} placeholder="org/repo"></textarea>
      </label>
      <label>
        <span>Filter orgs override</span>
        <textarea bind:value={filterOrgsText} placeholder="Advanced provider filter"></textarea>
      </label>
      <label>
        <span>Filter exclude override</span>
        <textarea bind:value={filterExcludeText} placeholder="Advanced provider filter"></textarea>
      </label>
      <label>
        <span>Include archived</span>
        <input class="check" type="checkbox" bind:checked={includeArchived} />
      </label>
      <label>
        <span>Include forks</span>
        <input class="check" type="checkbox" bind:checked={includeForks} />
      </label>
    </section>

    <section class="panel fields">
      <h2>Overrides</h2>
      <label>
        <span>Structure</span>
        <input bind:value={structure} placeholder={'Inherit global: {org}/{repo}'} />
      </label>
      <label>
        <span>Sync mode</span>
        <select bind:value={syncMode}>
          <option value="">Inherit global</option>
          <option value="fetch">Fetch</option>
          <option value="pull">Pull</option>
        </select>
      </label>
      <label>
        <span>Concurrency</span>
        <input bind:value={concurrencyText} inputmode="numeric" placeholder="Inherit global" />
      </label>
      <label>
        <span>Refresh interval</span>
        <input bind:value={refreshIntervalText} inputmode="numeric" placeholder="Inherit global" />
      </label>
      <label>
        <span>Clone override</span>
        <input class="check" type="checkbox" bind:checked={useCloneOverride} />
      </label>
      {#if useCloneOverride}
        <label>
          <span>Clone depth</span>
          <input type="number" min="0" bind:value={cloneDepth} />
        </label>
        <label>
          <span>Clone branch</span>
          <input bind:value={cloneBranch} placeholder="Default branch" />
        </label>
        <label>
          <span>Clone submodules</span>
          <input class="check" type="checkbox" bind:checked={cloneSubmodules} />
        </label>
      {/if}
    </section>
  </form>
  {/if}
</section>

<style>
  .workspace-screen {
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

  .workspace-form {
    display: grid;
    gap: 16px;
  }

  .panel {
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: var(--shadow);
    padding: 16px;
  }

  .hero {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: center;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    font-size: 15px;
  }

  p {
    margin-top: 4px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }

  .actions,
  .input-action {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .fields {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .fields h2,
  .wide {
    grid-column: 1 / -1;
  }

  label {
    min-width: 0;
    display: grid;
    gap: 6px;
  }

  label span {
    color: var(--muted);
    font-size: 12px;
    font-weight: 800;
  }

  input,
  select,
  textarea {
    width: 100%;
    min-width: 0;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-alt);
    color: var(--text);
    padding: 0 10px;
    font: inherit;
  }

  input,
  select {
    height: 34px;
  }

  textarea {
    min-height: 96px;
    padding: 9px 10px;
    resize: vertical;
  }

  input:disabled {
    opacity: 0.7;
  }

  .check {
    width: 18px;
    height: 18px;
    padding: 0;
  }

  .input-action {
    align-items: stretch;
  }

  .input-action input {
    flex: 1;
  }

  .workspace-form button {
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-alt);
    color: var(--text);
    cursor: pointer;
    padding: 0 12px;
    font-weight: 700;
    white-space: nowrap;
  }

  .workspace-form button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .workspace-form .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .workspace-form .danger {
    color: var(--danger);
  }

  .workspace-form .secondary {
    width: fit-content;
  }

  @media (max-width: 820px) {
    .hero {
      align-items: flex-start;
      flex-direction: column;
    }

    .fields {
      grid-template-columns: 1fr;
    }

    .actions,
    .input-action {
      flex-wrap: wrap;
    }
  }
</style>
