import { derived, get, writable } from 'svelte/store';
import {
  checkRequirements,
  deleteWorkspace,
  ensureConfig,
  installMonitorLaunchAgent,
  listWorkspaces,
  onStatusUpdated,
  onSyncProgress,
  readAppConfig,
  readExtensionStatus,
  readStatus,
  readWorkspaceStructure,
  restartMonitorLaunchAgent,
  saveAppConfig,
  setDefaultWorkspace,
  startSync,
} from '../lib/tauri';
import type {
  AppConfigDto,
  AppConfigInput,
  ExtensionStatus,
  ProgressEvent,
  RequirementCheckDto,
  StatusSnapshot,
  SyncProgressPayload,
  SyncProgressState,
  WorkspaceInput,
  WorkspaceStructureDto,
  WorkspaceSummary,
} from '../lib/types';
import { saveWorkspace as saveWorkspaceCommand } from '../lib/tauri';

export const NEW_WORKSPACE_ID = '__new_workspace__';

export const snapshot = writable<StatusSnapshot | null>(null);
export const workspaces = writable<WorkspaceSummary[]>([]);
export const extensionStatus = writable<ExtensionStatus | null>(null);
export const appConfig = writable<AppConfigDto | null>(null);
export const requirements = writable<RequirementCheckDto[]>([]);
export const workspaceStructure = writable<WorkspaceStructureDto | null>(null);
export const workspaceStructureLoading = writable<boolean>(false);
export const selectedWorkspaceId = writable<string>('');
export const loading = writable<boolean>(true);
export const requirementsLoading = writable<boolean>(false);
export const syncingId = writable<string>('');
export const errorMessage = writable<string>('');
export const successMessage = writable<string>('');
export const syncProgress = writable<SyncProgressState | null>(null);

export const currentWorkspace = derived(
  [workspaces, selectedWorkspaceId],
  ([$workspaces, $selectedWorkspaceId]) => {
    if ($selectedWorkspaceId === NEW_WORKSPACE_ID) return undefined;
    if ($selectedWorkspaceId) {
      return $workspaces.find((workspace) => workspace.id === $selectedWorkspaceId);
    }
    return $workspaces.find((workspace) => workspace.default) ?? $workspaces[0];
  },
);

export async function refresh(): Promise<void> {
  errorMessage.set('');
  const [workspaceList, status, ext, config] = await Promise.all([
    listWorkspaces().catch((err) => {
      errorMessage.set(String(err));
      return [] as WorkspaceSummary[];
    }),
    readStatus().catch((err) => {
      errorMessage.set(String(err));
      return null;
    }),
    readExtensionStatus().catch(() => null),
    readAppConfig().catch(() => null),
  ]);
  workspaces.set(workspaceList);
  snapshot.set(status);
  extensionStatus.set(ext);
  appConfig.set(config);
  reconcileSelectedWorkspace(workspaceList);
}

export async function loadAppConfig(): Promise<void> {
  appConfig.set(await readAppConfig());
}

export async function createDefaultConfig(): Promise<void> {
  appConfig.set(await ensureConfig());
  await refresh();
}

export async function saveConfig(input: AppConfigInput): Promise<void> {
  errorMessage.set('');
  appConfig.set(await saveAppConfig(input));
  successMessage.set('Settings saved');
  await refresh();
}

export async function saveWorkspace(input: WorkspaceInput): Promise<string> {
  errorMessage.set('');
  const saved = await saveWorkspaceCommand(input);
  selectedWorkspaceId.set(saved.id);
  successMessage.set('Workspace saved');
  await refresh();
  await loadCurrentWorkspaceStructure();
  return saved.id;
}

export async function removeWorkspace(workspaceId: string): Promise<void> {
  errorMessage.set('');
  const next = await deleteWorkspace(workspaceId);
  workspaces.set(next);
  selectedWorkspaceId.set(next.find((workspace) => workspace.default)?.id ?? next[0]?.id ?? '');
  successMessage.set('Workspace metadata removed');
  await refresh();
  await loadCurrentWorkspaceStructure();
}

export async function updateDefaultWorkspace(workspaceId: string | null): Promise<void> {
  errorMessage.set('');
  workspaces.set(await setDefaultWorkspace(workspaceId));
  await refresh();
}

export async function loadRequirements(): Promise<void> {
  requirementsLoading.set(true);
  errorMessage.set('');
  try {
    requirements.set(await checkRequirements());
  } catch (err) {
    errorMessage.set(String(err));
  } finally {
    requirementsLoading.set(false);
  }
}

export async function installMonitor(): Promise<void> {
  requirementsLoading.set(true);
  errorMessage.set('');
  try {
    await installMonitorLaunchAgent();
    successMessage.set('Monitor LaunchAgent installed');
    await Promise.all([refresh(), loadRequirements()]);
  } catch (err) {
    errorMessage.set(String(err));
  } finally {
    requirementsLoading.set(false);
  }
}

export async function restartMonitor(): Promise<void> {
  requirementsLoading.set(true);
  errorMessage.set('');
  try {
    await restartMonitorLaunchAgent();
    successMessage.set('Monitor LaunchAgent restarted');
    await Promise.all([refresh(), loadRequirements()]);
  } catch (err) {
    errorMessage.set(String(err));
  } finally {
    requirementsLoading.set(false);
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
    await loadCurrentWorkspaceStructure();
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

export async function loadCurrentWorkspaceStructure(): Promise<void> {
  const workspace = get(currentWorkspace);
  if (!workspace) {
    workspaceStructure.set(null);
    return;
  }
  workspaceStructureLoading.set(true);
  try {
    workspaceStructure.set(await readWorkspaceStructure(workspace.id));
  } catch (err) {
    errorMessage.set(String(err));
    workspaceStructure.set(null);
  } finally {
    workspaceStructureLoading.set(false);
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

function reconcileSelectedWorkspace(workspaceList: WorkspaceSummary[]) {
  const selected = get(selectedWorkspaceId);
  if (selected === NEW_WORKSPACE_ID) return;
  if (selected && workspaceList.some((workspace) => workspace.id === selected)) return;
  selectedWorkspaceId.set(
    workspaceList.find((workspace) => workspace.default)?.id ?? workspaceList[0]?.id ?? '',
  );
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
