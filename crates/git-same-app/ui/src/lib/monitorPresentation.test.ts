import { describe, expect, it } from 'vitest';
import {
  actionLabel,
  createStatusSequencer,
  presentMonitor,
  shouldSuggestFullDiskAccess,
} from './monitorPresentation';
import type { MonitorAgentState, MonitorAgentStatusDto } from './types';

function status(
  state: MonitorAgentState,
  extra: Partial<MonitorAgentStatusDto> = {},
): MonitorAgentStatusDto {
  return {
    label: 'com.zaai.git-same.monitor',
    plist_path: '',
    binary_path: null,
    installed: true,
    loaded: true,
    running: state === 'running' || state === 'starting',
    state,
    message: state,
    pid: null,
    mode: null,
    autostart: true,
    launchd_disabled: false,
    helper_version: null,
    source: null,
    owner_kind: null,
    last_scan: null,
    detail: null,
    ...extra,
  };
}

const ALL_STATES: MonitorAgentState[] = [
  'not_installed',
  'disabled',
  'deferred',
  'starting',
  'running',
  'stopped',
  'failed',
  'unsupported',
];

describe('presentMonitor', () => {
  it('handles every state and the not-yet-loaded case', () => {
    expect(presentMonitor(null).banner).toBe(false);
    for (const state of ALL_STATES) {
      expect(presentMonitor(status(state)).title).not.toBe('');
    }
  });

  it('falls back to a usable view for a state this build does not know', () => {
    // The helper can be a newer version than the app, so the TS union is a
    // description, not a guarantee. Falling off the switch used to return
    // undefined and blank the page on `view.title`.
    const view = presentMonitor(status('teleporting' as MonitorAgentState));

    expect(view).toBeDefined();
    expect(view.title).not.toBe('');
    expect(view.healthy).toBe(false);
    expect(view.tone).toBe('warning');
    expect(view.actions).toEqual(['restart']);
  });

  it('shows a long first scan as starting, never as broken', () => {
    const view = presentMonitor(status('starting', { pid: 4242 }));
    expect(view.title).toBe('Monitor is starting; initial scan in progress');
    expect(view.healthy).toBe(true);
    expect(view.tone).toBe('info');
    expect(view.actions).not.toContain('start');
  });

  it('shows PID and last completed scan while running', () => {
    const view = presentMonitor(
      status('running', { pid: 77, mode: 'managed', last_scan: '2026-09-20T10:00:00Z' }),
    );
    expect(view.detail).toContain('PID 77');
    expect(view.detail).toContain('2026-09-20T10:00:00Z');
    expect(view.actions).toEqual(['stop', 'restart']);
    expect(view.banner).toBe(false);
  });

  it('does not offer Restart for a monitor running in a terminal', () => {
    const view = presentMonitor(status('running', { mode: 'foreground' }));
    expect(view.actions).toEqual(['stop']);
  });

  it('shows an intentional stop as "Stopped by you" with Start', () => {
    const view = presentMonitor(status('disabled', { autostart: false }));
    expect(view.title).toBe('Stopped by you');
    expect(view.tone).toBe('info');
    expect(view.actions).toEqual(['start']);
  });

  it('explains a service disabled outside Git-Same', () => {
    const view = presentMonitor(
      status('disabled', { autostart: true, launchd_disabled: true }),
    );
    expect(view.title).toBe('Monitoring is disabled in macOS');
    expect(view.detail).toContain('launchctl');
    expect(view.actions).toEqual(['start']);
  });

  it('shows deferred as waiting for login, with nothing to do', () => {
    const view = presentMonitor(status('deferred'));
    expect(view.title).toBe('Will start when you log in');
    expect(view.actions).toEqual([]);
    expect(view.healthy).toBe(true);
  });

  it('shows the concrete error with a retry action when failed', () => {
    const failed = status('failed', { detail: 'launchctl bootstrap failed (exit 5)' });
    const view = presentMonitor(failed);
    expect(view.tone).toBe('error');
    expect(view.detail).toBe('launchctl bootstrap failed (exit 5)');
    expect(view.actions).toEqual(['start']);
    expect(actionLabel('start', failed)).toBe('Retry');
    expect(actionLabel('start', status('stopped'))).toBe('Start');
  });
});

describe('shouldSuggestFullDiskAccess', () => {
  const base = { extensionEnabled: true, workspaceCount: 2, repoCount: 0 };

  it('never warns while the initial scan is in progress', () => {
    expect(shouldSuggestFullDiskAccess({ ...base, status: status('starting') })).toBe(false);
    expect(shouldSuggestFullDiskAccess({ ...base, status: null })).toBe(false);
    expect(shouldSuggestFullDiskAccess({ ...base, status: status('stopped') })).toBe(false);
  });

  it('warns once a completed scan found nothing', () => {
    expect(shouldSuggestFullDiskAccess({ ...base, status: status('running') })).toBe(true);
    expect(
      shouldSuggestFullDiskAccess({ ...base, repoCount: 3, status: status('running') }),
    ).toBe(false);
  });
});

describe('createStatusSequencer', () => {
  it('accepts a fetch when no event arrived meanwhile', () => {
    const sequencer = createStatusSequencer<string>();
    const token = sequencer.beginFetch();
    expect(sequencer.acceptFetch(token, 'fetched')).toBe('fetched');
  });

  it('keeps an event that arrived while the fetch was in flight', () => {
    const sequencer = createStatusSequencer<string>();
    const token = sequencer.beginFetch();
    expect(sequencer.acceptEvent('pushed')).toBe('pushed');
    expect(sequencer.acceptFetch(token, 'older fetch')).toBeUndefined();
  });

  it('accepts a fetch started after the last event', () => {
    const sequencer = createStatusSequencer<string>();
    sequencer.acceptEvent('startup recovery');
    const token = sequencer.beginFetch();
    expect(sequencer.acceptFetch(token, 'fetched later')).toBe('fetched later');
  });
});
