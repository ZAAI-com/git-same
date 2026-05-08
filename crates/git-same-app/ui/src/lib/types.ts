export type Badge = 'green' | 'blue' | 'orange' | 'red' | 'gray';

export type SyncMode = 'fetch' | 'pull';

export interface WorkspaceSummary {
  id: string;
  name: string;
  root: string;
  provider: string;
  org_count: number;
  last_sync: string | null;
  default: boolean;
}

export interface CloneOptionsDto {
  depth: number;
  branch: string;
  recurse_submodules: boolean;
}

export interface FilterOptionsDto {
  include_archived: boolean;
  include_forks: boolean;
  orgs: string[];
  exclude_repos: string[];
}

export interface FinderConfigDto {
  scan_roots: string[];
  max_depth: number;
  exclude_dirs: string[];
  show_ambient: boolean;
}

export interface AppConfigDto {
  config_path: string;
  exists: boolean;
  structure: string;
  concurrency: number;
  sync_mode: SyncMode;
  default_workspace: string | null;
  refresh_interval: number;
  clone: CloneOptionsDto;
  filters: FilterOptionsDto;
  workspaces: string[];
  finder: FinderConfigDto;
}

export type AppConfigInput = Omit<AppConfigDto, 'config_path' | 'exists'>;

export interface WorkspaceProviderDto {
  kind: string;
  label: string;
  api_url: string | null;
  prefer_ssh: boolean;
}

export interface WorkspaceDetailDto {
  id: string;
  name: string;
  root: string;
  config_path: string;
  provider: WorkspaceProviderDto;
  username: string;
  orgs: string[];
  include_repos: string[];
  exclude_repos: string[];
  structure: string | null;
  sync_mode: SyncMode | null;
  clone_options: CloneOptionsDto | null;
  filters: FilterOptionsDto;
  concurrency: number | null;
  refresh_interval: number | null;
  last_synced: string | null;
  default: boolean;
}

export interface WorkspaceInput {
  id: string | null;
  root: string;
  provider: WorkspaceProviderDto;
  username: string;
  orgs: string[];
  include_repos: string[];
  exclude_repos: string[];
  structure: string | null;
  sync_mode: SyncMode | null;
  clone_options: CloneOptionsDto | null;
  filters: FilterOptionsDto;
  concurrency: number | null;
  refresh_interval: number | null;
  default: boolean;
}

export interface RequirementCheckDto {
  name: string;
  passed: boolean;
  message: string;
  suggestion: string | null;
  critical: boolean;
}

export interface MonitorLaunchAgentStatusDto {
  label: string;
  plist_path: string;
  binary_path: string | null;
  installed: boolean;
  loaded: boolean;
  running: boolean;
  state: string;
  message: string;
}

export interface ProviderOrgDto {
  name: string;
  repo_count: number;
  selected: boolean;
}

export interface ProviderDiscoveryDto {
  username: string | null;
  orgs: ProviderOrgDto[];
}

export interface WorkspaceStructureRepoDto {
  owner: string;
  name: string;
  full_name: string;
  url: string;
  local_path: string;
  local_exists: boolean;
}

export interface WorkspaceStructureDto {
  workspace_id: string;
  name: string;
  root: string;
  provider: string;
  host: string;
  source: 'cache' | 'remote' | 'unavailable' | string;
  cache_age_secs: number | null;
  error: string | null;
  repos: WorkspaceStructureRepoDto[];
}

export interface FinderWorkspaceInfo {
  name: string;
  root: string;
  orgs: string[];
}

export interface FinderBranchInfo {
  name: string;
  upstream?: string;
  ahead: number;
  behind: number;
  synced: boolean;
}

export interface FinderRemoteInfo {
  name: string;
  url: string;
}

export interface FinderWorktreeInfo {
  path: string;
  branch?: string;
  synced: boolean;
}

export interface OrgFolderInfo {
  path: string;
  org: string;
  workspace: string;
  owner_type: 'user' | 'organization' | 'unknown';
}

export interface FinderRepoStatus {
  path: string;
  workspace?: string;
  org?: string;
  badge: Badge;
  current_branch: string;
  default_branch?: string;
  commit_count: number;
  staged_count: number;
  unstaged_count: number;
  untracked_count: number;
  ahead: number;
  behind: number;
  stash_count: number;
  has_important_ignored_files: boolean;
  important_ignored_files?: string[];
  branches: FinderBranchInfo[];
  all_branches_synced: boolean;
  remotes: FinderRemoteInfo[];
  worktrees: FinderWorktreeInfo[];
  all_worktrees_synced: boolean;
  read_error?: string;
}

export interface FinderStatus {
  version: number;
  timestamp: string;
  daemon_pid: number;
  workspaces: FinderWorkspaceInfo[];
  custom_folders?: string[];
  repos: FinderRepoStatus[];
  org_folders?: OrgFolderInfo[];
  monitored_roots?: string[];
}

export interface StatusSnapshot {
  status_path: string;
  updated_at: string | null;
  stale: boolean;
  status: FinderStatus | null;
}

export interface ExtensionStatus {
  installed: boolean;
  enabled: boolean;
}

export type ProgressEvent =
  | { type: 'discovery_orgs_discovered'; count: number }
  | { type: 'discovery_org_started'; org_name: string }
  | { type: 'discovery_org_complete'; org_name: string; repo_count: number }
  | { type: 'discovery_personal_repos_started' }
  | { type: 'discovery_personal_repos_complete'; count: number }
  | { type: 'discovery_error'; message: string }
  | { type: 'clone_started'; repo_name: string; index: number; total: number }
  | { type: 'clone_completed'; repo_name: string; index: number; total: number }
  | {
      type: 'clone_failed';
      repo_name: string;
      error: string;
      index: number;
      total: number;
    }
  | {
      type: 'clone_skipped';
      repo_name: string;
      reason: string;
      index: number;
      total: number;
    }
  | {
      type: 'sync_started';
      repo_name: string;
      path: string;
      index: number;
      total: number;
    }
  | {
      type: 'sync_fetched';
      repo_name: string;
      updated: boolean;
      new_commits: number | null;
      index: number;
      total: number;
    }
  | {
      type: 'sync_pulled';
      repo_name: string;
      success: boolean;
      updated: boolean;
      fast_forward: boolean;
      error: string | null;
      index: number;
      total: number;
    }
  | {
      type: 'sync_failed';
      repo_name: string;
      error: string;
      index: number;
      total: number;
    }
  | {
      type: 'sync_skipped';
      repo_name: string;
      reason: string;
      index: number;
      total: number;
    };

export interface SyncProgressPayload {
  workspace_id: string;
  event: ProgressEvent;
}

export interface SyncProgressState {
  workspaceId: string;
  message: string;
  completed: number;
  total: number | null;
  failed: number;
  skipped: number;
}
