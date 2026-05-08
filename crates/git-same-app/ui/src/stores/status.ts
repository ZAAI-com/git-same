import { derived, get, writable } from 'svelte/store';
import {
  listWorkspaces,
  onStatusUpdated,
  readStatus,
  startSync,
} from '../lib/tauri';
import type { StatusSnapshot, WorkspaceSummary } from '../lib/types';

export const snapshot = writable<StatusSnapshot | null>(null);
export const workspaces = writable<WorkspaceSummary[]>([]);
export const selectedWorkspaceId = writable<string>('');
export const loading = writable<boolean>(true);
export const syncingId = writable<string>('');
export const errorMessage = writable<string>('');

export const currentWorkspace = derived(
  [workspaces, selectedWorkspaceId],
  ([$workspaces, $selectedWorkspaceId]) =>
    $workspaces.find((workspace) => workspace.id === $selectedWorkspaceId) ??
    $workspaces[0],
);

export async function refresh(): Promise<void> {
  errorMessage.set('');
  const [workspaceList, status] = await Promise.all([
    listWorkspaces(),
    readStatus(),
  ]);
  workspaces.set(workspaceList);
  snapshot.set(status);
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
  try {
    await startSync(workspace.id);
    await refresh();
  } catch (err) {
    errorMessage.set(String(err));
  } finally {
    syncingId.set('');
  }
}

export async function subscribePush(): Promise<() => void> {
  return onStatusUpdated((next) => {
    snapshot.set(next);
  });
}
