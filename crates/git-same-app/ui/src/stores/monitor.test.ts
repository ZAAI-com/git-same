import { get } from 'svelte/store';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MonitorAgentStatusDto } from '../lib/types';

const api = vi.hoisted(() => ({
  listener: undefined as ((status: MonitorAgentStatusDto) => void) | undefined,
  monitorStatus: vi.fn(),
  startMonitor: vi.fn(),
  stopMonitor: vi.fn(),
  restartMonitor: vi.fn(),
}));

vi.mock('../lib/tauri', () => ({
  monitorStatus: api.monitorStatus,
  startMonitor: api.startMonitor,
  stopMonitor: api.stopMonitor,
  restartMonitor: api.restartMonitor,
  onMonitorAgentUpdated: async (callback: (status: MonitorAgentStatusDto) => void) => {
    api.listener = callback;
    return () => {
      api.listener = undefined;
    };
  },
}));

const make = (state: MonitorAgentStatusDto['state']) =>
  ({ state, message: state }) as MonitorAgentStatusDto;

async function freshStore() {
  vi.resetModules();
  return import('./monitor');
}

beforeEach(() => {
  vi.clearAllMocks();
  api.listener = undefined;
});

describe('monitor store', () => {
  it('keeps an event that arrives before the initial fetch resolves', async () => {
    const store = await freshStore();
    let resolveFetch!: (status: MonitorAgentStatusDto) => void;
    api.monitorStatus.mockReturnValue(new Promise((resolve) => (resolveFetch = resolve)));

    await store.subscribeMonitor();
    const loading = store.loadMonitorStatus();
    api.listener?.(make('running'));
    resolveFetch(make('stopped'));
    await loading;

    expect(get(store.monitorStatus)?.state).toBe('running');
  });

  it('uses the fetched status when no event arrived, then follows events', async () => {
    const store = await freshStore();
    api.monitorStatus.mockResolvedValue(make('starting'));

    await store.subscribeMonitor();
    await store.loadMonitorStatus();
    expect(get(store.monitorStatus)?.state).toBe('starting');

    api.listener?.(make('running'));
    expect(get(store.monitorStatus)?.state).toBe('running');
  });

  it('marks the store busy during an action and applies its result', async () => {
    const store = await freshStore();
    let resolveStop!: (status: MonitorAgentStatusDto) => void;
    api.stopMonitor.mockReturnValue(new Promise((resolve) => (resolveStop = resolve)));

    const stopping = store.runMonitorAction('stop');
    expect(get(store.monitorBusy)).toBe(true);
    resolveStop(make('disabled'));
    await stopping;

    expect(get(store.monitorBusy)).toBe(false);
    expect(get(store.monitorStatus)?.state).toBe('disabled');
  });

  it('ignores a duplicate action while one is in flight', async () => {
    const store = await freshStore();
    let resolveStart!: (status: MonitorAgentStatusDto) => void;
    api.startMonitor.mockReturnValue(new Promise((resolve) => (resolveStart = resolve)));

    const first = store.runMonitorAction('start');
    await store.runMonitorAction('start');
    await store.runMonitorAction('restart');
    resolveStart(make('starting'));
    await first;

    expect(api.startMonitor).toHaveBeenCalledTimes(1);
    expect(api.restartMonitor).not.toHaveBeenCalled();
  });

  it('surfaces a failed action and reloads the real status', async () => {
    const store = await freshStore();
    api.startMonitor.mockRejectedValue('A foreground monitor (PID 9) is running');
    api.monitorStatus.mockResolvedValue(make('running'));

    await store.runMonitorAction('start');

    expect(get(store.monitorError)).toContain('foreground monitor');
    expect(get(store.monitorStatus)?.state).toBe('running');
    expect(get(store.monitorBusy)).toBe(false);
  });

  it('keeps the action error while reloading after that action failed', async () => {
    const store = await freshStore();
    api.startMonitor.mockRejectedValue('A foreground monitor (PID 9) is running');
    api.monitorStatus.mockResolvedValue(make('running'));

    await store.runMonitorAction('start');

    // The recovery fetch succeeding says nothing about why Start failed.
    expect(get(store.monitorError)).toContain('foreground monitor');
  });

  it('keeps the action error when its recovery fetch also fails', async () => {
    const store = await freshStore();
    api.startMonitor.mockRejectedValue('Start failed for the real reason');
    api.monitorStatus.mockRejectedValue('Recovery fetch failed');

    await store.runMonitorAction('start');

    expect(get(store.monitorError)).toContain('real reason');
    expect(get(store.monitorError)).not.toContain('Recovery fetch');
  });

  it('clears a stale error once a later fetch succeeds', async () => {
    const store = await freshStore();
    api.startMonitor.mockRejectedValue('A foreground monitor (PID 9) is running');
    api.monitorStatus.mockResolvedValue(make('running'));

    await store.runMonitorAction('start');
    expect(get(store.monitorError)).not.toBe('');

    await store.loadMonitorStatus();

    expect(get(store.monitorError)).toBe('');
    expect(get(store.monitorStatus)?.state).toBe('running');
  });
});
