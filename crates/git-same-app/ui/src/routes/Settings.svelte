<script lang="ts">
  import { RotateCcw, Save } from 'lucide-svelte';
  import EmptyState from '../lib/EmptyState.svelte';
  import {
    appConfig,
    createDefaultConfig,
    errorMessage,
    saveConfig,
  } from '../stores/status';
  import { linesToList, listToLines } from '../lib/utils';
  import type { AppConfigDto, AppConfigInput, SyncMode } from '../lib/types';

  let loadedPath = '';
  let saving = false;
  let structure = '{org}/{repo}';
  let concurrency = 4;
  let syncMode: SyncMode = 'fetch';
  let defaultWorkspace = '';
  let refreshInterval = 30;
  let cloneDepth = 0;
  let cloneBranch = '';
  let cloneSubmodules = false;
  let includeArchived = false;
  let includeForks = false;
  let filterOrgsText = '';
  let filterExcludeText = '';
  let workspacesText = '';
  let showAmbient = true;
  let scanRootsText = '~';
  let finderMaxDepth = 8;
  let finderExcludeText =
    'node_modules\ntarget\nbuild\ndist\nDerivedData\nPods\nLibrary\n.cache\n.cargo\n.rustup\n.npm\n.yarn\n.venv\n.Trash\n.git-same\n.zsh_sessions';
  let monitorFullscanInterval = 30;

  $: if ($appConfig && $appConfig.config_path !== loadedPath) {
    applyConfig($appConfig);
  }

  function applyConfig(config: AppConfigDto) {
    loadedPath = config.config_path;
    structure = config.structure;
    concurrency = config.concurrency;
    syncMode = config.sync_mode;
    defaultWorkspace = config.default_workspace ?? '';
    refreshInterval = config.refresh_interval;
    cloneDepth = config.clone.depth;
    cloneBranch = config.clone.branch;
    cloneSubmodules = config.clone.recurse_submodules;
    includeArchived = config.filters.include_archived;
    includeForks = config.filters.include_forks;
    filterOrgsText = listToLines(config.filters.orgs);
    filterExcludeText = listToLines(config.filters.exclude_repos);
    workspacesText = listToLines(config.workspaces);
    showAmbient = config.finder.show_ambient;
    scanRootsText = listToLines(config.finder.scan_roots);
    finderMaxDepth = config.finder.max_depth;
    finderExcludeText = listToLines(config.finder.exclude_dirs);
    monitorFullscanInterval = config.monitor.fullscan_interval_secs;
  }

  function restoreDefaults() {
    structure = '{org}/{repo}';
    concurrency = 4;
    syncMode = 'fetch';
    defaultWorkspace = '';
    refreshInterval = 30;
    cloneDepth = 0;
    cloneBranch = '';
    cloneSubmodules = false;
    includeArchived = false;
    includeForks = false;
    filterOrgsText = '';
    filterExcludeText = '';
    showAmbient = true;
    scanRootsText = '~';
    finderMaxDepth = 8;
    finderExcludeText =
      'node_modules\ntarget\nbuild\ndist\nDerivedData\nPods\nLibrary\n.cache\n.cargo\n.rustup\n.npm\n.yarn\n.venv\n.Trash\n.git-same\n.zsh_sessions';
    monitorFullscanInterval = 30;
  }

  async function submit() {
    saving = true;
    try {
      const input: AppConfigInput = {
        structure,
        concurrency: Number(concurrency) || 4,
        sync_mode: syncMode,
        default_workspace: defaultWorkspace.trim() || null,
        refresh_interval: Number(refreshInterval) || 30,
        clone: {
          depth: Number(cloneDepth) || 0,
          branch: cloneBranch.trim(),
          recurse_submodules: cloneSubmodules,
        },
        filters: {
          include_archived: includeArchived,
          include_forks: includeForks,
          orgs: linesToList(filterOrgsText),
          exclude_repos: linesToList(filterExcludeText),
        },
        workspaces: linesToList(workspacesText),
        finder: {
          scan_roots: linesToList(scanRootsText),
          max_depth: Number(finderMaxDepth) || 8,
          exclude_dirs: linesToList(finderExcludeText),
          show_ambient: showAmbient,
        },
        monitor: {
          fullscan_interval_secs: Number(monitorFullscanInterval) || 30,
        },
      };
      await saveConfig(input);
    } catch (err) {
      errorMessage.set(String(err));
    } finally {
      saving = false;
    }
  }
</script>

{#if !$appConfig}
  <EmptyState title="Config unavailable" detail="Create the default config before editing global settings.">
    <button type="button" on:click={createDefaultConfig}>Create Config</button>
  </EmptyState>
{:else if !$appConfig.exists}
  <EmptyState title="Config file not created" detail="Git-Same can create a default config at the standard path.">
    <button type="button" on:click={createDefaultConfig}>Create Config</button>
  </EmptyState>
{:else}
  <form class="settings-form" on:submit|preventDefault={submit}>
    <section class="panel hero">
      <div>
        <h2>Global Config</h2>
        <p>{$appConfig.config_path}</p>
      </div>
      <div class="actions">
        <button class="secondary" type="button" on:click={restoreDefaults}>
          <RotateCcw size={16} />
          <span>Defaults</span>
        </button>
        <button class="primary" type="submit" disabled={saving}>
          <Save size={16} />
          <span>{saving ? 'Saving' : 'Save'}</span>
        </button>
      </div>
    </section>

    <section class="panel fields">
      <h2>Sync Defaults</h2>
      <label>
        <span>Directory structure</span>
        <input bind:value={structure} />
      </label>
      <label>
        <span>Concurrency</span>
        <input type="number" min="1" max="32" bind:value={concurrency} />
      </label>
      <label>
        <span>Sync mode</span>
        <select bind:value={syncMode}>
          <option value="fetch">Fetch</option>
          <option value="pull">Pull</option>
        </select>
      </label>
      <label>
        <span>Refresh interval</span>
        <input type="number" min="5" max="3600" bind:value={refreshInterval} />
      </label>
      <label>
        <span>Default workspace</span>
        <input bind:value={defaultWorkspace} placeholder="Optional workspace path" />
      </label>
      <label>
        <span>Workspace registry</span>
        <textarea bind:value={workspacesText} placeholder="One workspace path per line"></textarea>
      </label>
    </section>

    <section class="panel fields">
      <h2>Clone Defaults</h2>
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
    </section>

    <section class="panel fields">
      <h2>Provider Filters</h2>
      <label>
        <span>Include archived</span>
        <input class="check" type="checkbox" bind:checked={includeArchived} />
      </label>
      <label>
        <span>Include forks</span>
        <input class="check" type="checkbox" bind:checked={includeForks} />
      </label>
      <label>
        <span>Organizations</span>
        <textarea bind:value={filterOrgsText} placeholder="One org per line; empty means all"></textarea>
      </label>
      <label>
        <span>Exclude repos</span>
        <textarea bind:value={filterExcludeText} placeholder="org/repo"></textarea>
      </label>
    </section>

    <section class="panel fields">
      <h2>Monitor</h2>
      <label>
        <span>Fullscan interval (seconds)</span>
        <input type="number" min="5" max="3600" bind:value={monitorFullscanInterval} />
      </label>
      <p class="hint">Restart the monitor on the Requirements screen for changes to take effect.</p>
    </section>

    <section class="panel fields">
      <h2>Finder Ambient Badges</h2>
      <label>
        <span>Show ambient repos</span>
        <input class="check" type="checkbox" bind:checked={showAmbient} />
      </label>
      <label>
        <span>Max depth</span>
        <input type="number" min="1" bind:value={finderMaxDepth} />
      </label>
      <label>
        <span>Scan roots</span>
        <textarea bind:value={scanRootsText}></textarea>
      </label>
      <label>
        <span>Excluded dirs</span>
        <textarea bind:value={finderExcludeText}></textarea>
      </label>
    </section>
  </form>
{/if}

<style>
  .settings-form {
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

  .actions {
    display: flex;
    gap: 8px;
  }

  .fields {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .fields h2 {
    grid-column: 1 / -1;
  }

  .fields .hint {
    grid-column: 1 / -1;
    font-size: 12px;
    color: var(--muted);
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
    min-height: 98px;
    padding: 9px 10px;
    resize: vertical;
  }

  .check {
    width: 18px;
    height: 18px;
    padding: 0;
  }

  button {
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
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  @media (max-width: 820px) {
    .hero {
      align-items: flex-start;
      flex-direction: column;
    }

    .fields {
      grid-template-columns: 1fr;
    }
  }
</style>
