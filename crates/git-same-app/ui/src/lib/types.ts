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
