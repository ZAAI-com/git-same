import { derived, get, writable } from 'svelte/store';
import {
  listWorkspaces,
  onStatusUpdated,
  onSyncProgress,
  readExtensionStatus,
  readStatus,
  startSync,
} from '../lib/tauri';
import type {
  ExtensionStatus,
  ProgressEvent,
  StatusSnapshot,
  SyncProgressPayload,
  SyncProgressState,
  WorkspaceSummary,
} from '../lib/types';

export const snapshot = writable<StatusSnapshot | null>(null);
export const workspaces = writable<WorkspaceSummary[]>([]);
export const extensionStatus = writable<ExtensionStatus | null>(null);
export const selectedWorkspaceId = writable<string>('');
export const loading = writable<boolean>(true);
export const syncingId = writable<string>('');
export const errorMessage = writable<string>('');
export const syncProgress = writable<SyncProgressState | null>(null);

export const currentWorkspace = derived(
  [workspaces, selectedWorkspaceId],
  ([$workspaces, $selectedWorkspaceId]) =>
    $workspaces.find((workspace) => workspace.id === $selectedWorkspaceId) ??
    $workspaces[0],
);

export async function refresh(): Promise<void> {
  errorMessage.set('');
  const [workspaceList, status, ext] = await Promise.all([
    listWorkspaces(),
    readStatus(),
    readExtensionStatus().catch(() => null),
  ]);
  workspaces.set(workspaceList);
  snapshot.set(status);
  extensionStatus.set(ext);
  if (!get(selectedWorkspaceId) && workspaceList.length > 0) {
    const defaultId =
      workspaceList.find((workspace) => workspace.default)?.id ??
      workspaceList[0].id;
    selectedWorkspaceId.set(defaultId);
  }
}

export async function startSyncCurrent(): Promise<void> {
  const workspace = get(currentWorkspace);
  if (!workspace) return;
  syncingId.set(workspace.id);
  errorMessage.set('');
  syncProgress.set({
    workspaceId: workspace.id,
    message: 'Starting sync',
    completed: 0,
    total: null,
    failed: 0,
    skipped: 0,
  });
  try {
    const next = await startSync(workspace.id);
    snapshot.set(next);
    await refresh();
  } catch (err) {
    errorMessage.set(String(err));
  } finally {
    syncingId.set('');
    window.setTimeout(() => {
      syncProgress.update((current) =>
        current?.workspaceId === workspace.id ? null : current,
      );
    }, 1200);
  }
}

export async function subscribePush(): Promise<() => void> {
  const unsubscribeStatus = await onStatusUpdated((next) => {
    snapshot.set(next);
  });
  const unsubscribeProgress = await onSyncProgress((payload) => {
    syncProgress.update((current) => reduceSyncProgress(current, payload));
  });
  return () => {
    unsubscribeStatus();
    unsubscribeProgress();
  };
}

function reduceSyncProgress(
  current: SyncProgressState | null,
  payload: SyncProgressPayload,
): SyncProgressState {
  const event = payload.event;
  const next: SyncProgressState =
    current?.workspaceId === payload.workspace_id
      ? { ...current }
      : {
          workspaceId: payload.workspace_id,
          message: 'Starting sync',
          completed: 0,
          total: null,
          failed: 0,
          skipped: 0,
        };

  next.message = progressMessage(event);
  next.total = progressTotal(event) ?? next.total;
  next.completed = Math.max(next.completed, progressCompleted(event));
  if (isFailure(event)) next.failed += 1;
  if (isSkip(event)) next.skipped += 1;
  return next;
}

function progressMessage(event: ProgressEvent): string {
  switch (event.type) {
    case 'discovery_orgs_discovered':
      return `Found ${event.count} organizations`;
    case 'discovery_org_started':
      return `Discovering ${event.org_name}`;
    case 'discovery_org_complete':
      return `Found ${event.repo_count} repos in ${event.org_name}`;
    case 'discovery_personal_repos_started':
      return 'Discovering personal repos';
    case 'discovery_personal_repos_complete':
      return `Found ${event.count} personal repos`;
    case 'discovery_error':
      return event.message;
    case 'clone_started':
      return `Cloning ${event.repo_name}`;
    case 'clone_completed':
      return `Cloned ${event.repo_name}`;
    case 'clone_failed':
      return `Clone failed: ${event.repo_name}`;
    case 'clone_skipped':
      return `Skipped clone: ${event.repo_name}`;
    case 'sync_started':
      return `Syncing ${event.repo_name}`;
    case 'sync_fetched':
      return event.updated
        ? `Fetched ${event.repo_name}`
        : `Checked ${event.repo_name}`;
    case 'sync_pulled':
      return `Pulled ${event.repo_name}`;
    case 'sync_failed':
      return `Sync failed: ${event.repo_name}`;
    case 'sync_skipped':
      return `Skipped sync: ${event.repo_name}`;
  }
}

function progressTotal(event: ProgressEvent): number | null {
  return 'total' in event ? event.total : null;
}

function progressCompleted(event: ProgressEvent): number {
  if (!('index' in event)) return 0;
  return event.type.endsWith('started') ? 0 : event.index + 1;
}

function isFailure(event: ProgressEvent): boolean {
  return event.type === 'clone_failed' || event.type === 'sync_failed';
}

function isSkip(event: ProgressEvent): boolean {
  return event.type === 'clone_skipped' || event.type === 'sync_skipped';
}
