import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type {
  AppConfigDto,
  AppConfigInput,
  ExtensionStatus,
  ProviderDiscoveryDto,
  RequirementCheckDto,
  StatusSnapshot,
  SyncProgressPayload,
  WorkspaceDetailDto,
  WorkspaceInput,
  WorkspaceProviderDto,
  WorkspaceSummary,
} from './types';

export function listWorkspaces(): Promise<WorkspaceSummary[]> {
  return invoke('list_workspaces');
}

export function readAppConfig(): Promise<AppConfigDto> {
  return invoke('read_app_config');
}

export function ensureConfig(): Promise<AppConfigDto> {
  return invoke('ensure_config');
}

export function saveAppConfig(input: AppConfigInput): Promise<AppConfigDto> {
  return invoke('save_app_config', { input });
}

export function readWorkspace(workspaceId: string): Promise<WorkspaceDetailDto> {
  return invoke('read_workspace', { workspaceId });
}

export function saveWorkspace(input: WorkspaceInput): Promise<WorkspaceDetailDto> {
  return invoke('save_workspace', { input });
}

export function deleteWorkspace(workspaceId: string): Promise<WorkspaceSummary[]> {
  return invoke('delete_workspace', { workspaceId });
}

export function setDefaultWorkspace(
  workspaceId: string | null,
): Promise<WorkspaceSummary[]> {
  return invoke('set_default_workspace', { workspaceId });
}

export function checkRequirements(): Promise<RequirementCheckDto[]> {
  return invoke('check_requirements');
}

export function discoverProviderOrgs(
  provider: WorkspaceProviderDto,
): Promise<ProviderDiscoveryDto> {
  return invoke('discover_provider_orgs', { provider });
}

export function readStatus(): Promise<StatusSnapshot> {
  return invoke('read_status');
}

export function startSync(workspaceId: string): Promise<StatusSnapshot> {
  return invoke('start_sync', { workspaceId });
}

export function readExtensionStatus(): Promise<ExtensionStatus> {
  return invoke('extension_status');
}

export function openUrl(url: string): Promise<void> {
  return invoke('open_url', { url });
}

export async function chooseFolder(defaultPath?: string): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath,
  });
  return typeof selected === 'string' ? selected : null;
}

export function onStatusUpdated(callback: (snapshot: StatusSnapshot) => void) {
  return listen<StatusSnapshot>('status-updated', (event) => callback(event.payload));
}

export function onSyncProgress(callback: (payload: SyncProgressPayload) => void) {
  return listen<SyncProgressPayload>('sync-progress', (event) => callback(event.payload));
}
