import { get } from 'svelte/store';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { StatusSnapshot } from '../lib/types';

const api = vi.hoisted(() => ({
  listener: undefined as ((snapshot: StatusSnapshot) => void) | undefined,
  readStatus: vi.fn(),
  listWorkspaces: vi.fn(),
  readExtensionStatus: vi.fn(),
  readFullDiskAccess: vi.fn(),
  readAppConfig: vi.fn(),
}));

vi.mock('../lib/tauri', () => ({
  readStatus: api.readStatus,
  listWorkspaces: api.listWorkspaces,
  readExtensionStatus: api.readExtensionStatus,
  readFullDiskAccess: api.readFullDiskAccess,
  readAppConfig: api.readAppConfig,
  onStatusUpdated: async (callback: (snapshot: StatusSnapshot) => void) => {
    api.listener = callback;
    return () => {
      api.listener = undefined;
    };
  },
  onSyncProgress: async () => () => {},
  // Unused by these tests, but `status.ts` imports `./monitor`, which pulls
  // the whole boundary module in, so every named export has to exist.
  monitorStatus: vi.fn(),
  startMonitor: vi.fn(),
  stopMonitor: vi.fn(),
  restartMonitor: vi.fn(),
  onMonitorAgentUpdated: async () => () => {},
  checkRequirements: vi.fn(),
  deleteWorkspace: vi.fn(),
  ensureConfig: vi.fn(),
  readWorkspaceStructure: vi.fn(),
  saveAppConfig: vi.fn(),
  setDefaultWorkspace: vi.fn(),
  startSync: vi.fn(),
  enableFinderExtension: vi.fn(),
  restartMonitorLaunchAgent: vi.fn(),
}));

const make = (updated_at: string | null, stale = false): StatusSnapshot => ({
  status_path: '/tmp/status.json',
  updated_at,
  stale,
  status: null,
});

async function freshStore() {
  vi.resetModules();
  return import('./status');
}

beforeEach(() => {
  vi.clearAllMocks();
  api.listener = undefined;
  api.listWorkspaces.mockResolvedValue([]);
  api.readExtensionStatus.mockResolvedValue(null);
  api.readFullDiskAccess.mockResolvedValue(null);
  api.readAppConfig.mockResolvedValue(null);
});

describe('status store snapshot ordering', () => {
  it('keeps a pushed snapshot that arrives while the fetch is in flight', async () => {
    const store = await freshStore();
    let resolveFetch!: (snapshot: StatusSnapshot | null) => void;
    api.readStatus.mockReturnValue(new Promise((resolve) => (resolveFetch = resolve)));

    await store.subscribePush();
    const refreshing = store.refresh();
    // The monitor's first scan lands before readStatus() resolves.
    api.listener?.(make('2026-09-20T10:00:01Z'));
    resolveFetch(make(null));
    await refreshing;

    expect(get(store.snapshot)?.updated_at).toBe('2026-09-20T10:00:01Z');
  });

  it('uses the fetched snapshot when no event arrived, then follows events', async () => {
    const store = await freshStore();
    api.readStatus.mockResolvedValue(make('2026-09-20T09:00:00Z'));

    await store.subscribePush();
    await store.refresh();
    expect(get(store.snapshot)?.updated_at).toBe('2026-09-20T09:00:00Z');

    api.listener?.(make('2026-09-20T09:05:00Z'));
    expect(get(store.snapshot)?.updated_at).toBe('2026-09-20T09:05:00Z');
  });

  it('still stores null when readStatus fails and nothing newer arrived', async () => {
    const store = await freshStore();
    api.readStatus.mockRejectedValue('status file unreadable');

    await store.subscribePush();
    await store.refresh();

    expect(get(store.snapshot)).toBeNull();
    expect(get(store.errorMessage)).toContain('unreadable');
  });
});
