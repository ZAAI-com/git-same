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
  restartMonitorIfAgentInstalled: vi.fn(),
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
  restartMonitorIfAgentInstalled: api.restartMonitorIfAgentInstalled,
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
  api.restartMonitorIfAgentInstalled.mockResolvedValue(null);
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

// Full Disk Access is granted to the app but the running monitor predates the
// grant: the store restarts it once so its scans pick the grant up.
const laggingFda = {
  host: 'granted',
  monitor: false,
  monitor_fresh: true,
  granted: false,
};

const managedAgent = { installed: true, mode: 'managed' };

// `monitorStatus` lives in ./monitor; after resetModules it must be imported
// from the same fresh graph status.ts is bound to, or the store instances differ.
async function freshStores(agent: unknown) {
  vi.resetModules();
  const monitor = await import('./monitor');
  const store = await import('./status');
  store.__resetMonitorKickForTests();
  monitor.monitorStatus.set(agent as never);
  return store;
}

describe('monitor kick on lagging Full Disk Access', () => {
  it('restarts a managed monitor and re-reads Full Disk Access', async () => {
    const store = await freshStores(managedAgent);
    // Lagging on the first read, granted once the monitor has restarted.
    api.readFullDiskAccess
      .mockResolvedValueOnce(laggingFda)
      .mockResolvedValueOnce({ ...laggingFda, monitor: true, granted: true });

    await store.refreshPermissions();

    expect(api.restartMonitorIfAgentInstalled).toHaveBeenCalledTimes(1);
    expect(get(store.fullDiskAccess)).toMatchObject({ monitor: true, granted: true });
  });

  it('never kicks a monitor the user started by hand', async () => {
    // Foreground: the backend refuses to kill it, so a kick only raises an
    // error on every refresh and focus.
    const store = await freshStores({ installed: false, mode: 'foreground' });
    api.readFullDiskAccess.mockResolvedValue(laggingFda);

    await store.refreshPermissions();

    expect(api.restartMonitorIfAgentInstalled).not.toHaveBeenCalled();
    expect(get(store.errorMessage)).toBe('');
  });

  it('never kicks when no service is installed', async () => {
    const store = await freshStores({ installed: false, mode: null });
    api.readFullDiskAccess.mockResolvedValue(laggingFda);

    await store.refreshPermissions();

    expect(api.restartMonitorIfAgentInstalled).not.toHaveBeenCalled();
  });

  it('retries after a failed restart instead of latching off recovery', async () => {
    const store = await freshStores(managedAgent);
    api.readFullDiskAccess.mockResolvedValue(laggingFda);
    api.restartMonitorIfAgentInstalled.mockRejectedValue(new Error('launchctl busy'));

    await store.refreshPermissions();
    await store.refreshPermissions();

    expect(api.restartMonitorIfAgentInstalled).toHaveBeenCalledTimes(2);
    expect(get(store.errorMessage)).toContain('launchctl busy');
  });

  it('kicks only once while a restart keeps succeeding', async () => {
    const store = await freshStores(managedAgent);
    api.readFullDiskAccess.mockResolvedValue(laggingFda);

    await store.refreshPermissions();
    await store.refreshPermissions();

    expect(api.restartMonitorIfAgentInstalled).toHaveBeenCalledTimes(1);
  });
});
