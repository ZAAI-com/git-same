export type Badge = 'green' | 'blue' | 'orange' | 'red' | 'gray';

export interface WorkspaceSummary {
  id: string;
  root: string;
  provider: string;
  org_count: number;
  last_sync: string | null;
  default: boolean;
}

export interface FinderWorkspaceInfo {
  name: string;
  root: string;
  orgs: string[];
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
  all_branches_synced: boolean;
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
