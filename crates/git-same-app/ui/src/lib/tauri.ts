import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { StatusSnapshot, WorkspaceSummary } from './types';

export function listWorkspaces(): Promise<WorkspaceSummary[]> {
  return invoke('list_workspaces');
}

export function readStatus(): Promise<StatusSnapshot> {
  return invoke('read_status');
}

export function startSync(workspaceId: string): Promise<void> {
  return invoke('start_sync', { workspaceId });
}

export function onStatusUpdated(callback: (snapshot: StatusSnapshot) => void) {
  return listen<StatusSnapshot>('status-updated', (event) => callback(event.payload));
}
