// Pure mapping from monitor service status to what the UI shows.
//
// Kept free of Svelte and Tauri so every state can be unit tested. Service
// state and badge-data freshness are separate: nothing here looks at how old
// the status file is.

import type { MonitorAgentStatusDto } from './types';

export type MonitorAction = 'start' | 'stop' | 'restart';
export type MonitorTone = 'ok' | 'info' | 'warning' | 'error';

export interface MonitorPresentation {
  tone: MonitorTone;
  title: string;
  detail: string | null;
  /** Controls that make sense in this state, primary first. */
  actions: MonitorAction[];
  /** The monitor works or is on its way; nothing for the user to fix. */
  healthy: boolean;
  /** Worth interrupting the user with a banner. */
  banner: boolean;
}

const UNKNOWN: MonitorPresentation = {
  tone: 'info',
  title: 'Checking the monitor',
  detail: null,
  actions: [],
  healthy: true,
  banner: false,
};

/**
 * A state this build does not know about. The DTO crosses an IPC boundary to
 * a helper that can be a different version than the app, so the TypeScript
 * union is a description of what we expect, not a guarantee. Without this the
 * switch below falls off the end and returns `undefined` while typed
 * non-nullable, and `MonitorPanel` throws on `view.title` and blanks the page.
 */
const UNRECOGNIZED: MonitorPresentation = {
  tone: 'warning',
  title: 'Monitor state not recognized',
  detail:
    'The background monitor reported a state this version of the app does not know. ' +
    'The app and the helper are probably different versions. Restarting the monitor ' +
    'usually reinstalls a matching helper.',
  actions: ['restart'],
  healthy: false,
  banner: true,
};

export function presentMonitor(status: MonitorAgentStatusDto | null): MonitorPresentation {
  if (!status) return UNKNOWN;

  switch (status.state) {
    case 'running':
      return {
        tone: 'ok',
        title:
          status.mode === 'foreground'
            ? 'Monitor is running in a terminal'
            : 'Monitor is running',
        detail: runningDetail(status),
        // A foreground monitor belongs to the terminal that started it.
        actions: status.mode === 'foreground' ? ['stop'] : ['stop', 'restart'],
        healthy: true,
        banner: false,
      };
    case 'starting':
      return {
        tone: 'info',
        title: 'Monitor is starting; initial scan in progress',
        detail: status.pid ? `PID ${status.pid}. Large workspaces can take a few minutes.` : null,
        actions: ['stop'],
        healthy: true,
        banner: true,
      };
    case 'disabled':
      // An intentional stop is not a broken installation.
      return status.autostart
        ? {
            tone: 'info',
            title: 'Monitoring is disabled in macOS',
            detail:
              'The background service was disabled outside Git-Same (launchctl). Start re-enables it.',
            actions: ['start'],
            healthy: false,
            banner: true,
          }
        : {
            tone: 'info',
            title: 'Stopped by you',
            detail: 'Finder badges are paused until you start monitoring again.',
            actions: ['start'],
            healthy: false,
            banner: true,
          };
    case 'deferred':
      return {
        tone: 'info',
        title: 'Will start when you log in',
        detail: 'The monitor is installed but this session has no desktop login.',
        actions: [],
        healthy: true,
        banner: false,
      };
    case 'not_installed':
      return {
        tone: 'warning',
        title: 'Background monitor is not installed',
        detail: status.detail,
        actions: ['start'],
        healthy: false,
        banner: true,
      };
    case 'stopped':
      return {
        tone: 'warning',
        title: 'Monitor is not running',
        detail: status.detail,
        actions: ['start'],
        healthy: false,
        banner: true,
      };
    case 'failed':
      return {
        tone: 'error',
        title: 'Monitor needs attention',
        detail: status.detail ?? status.message,
        actions: ['start'],
        healthy: false,
        banner: true,
      };
    case 'unsupported':
      return {
        tone: 'info',
        title: 'Background monitoring is only available on macOS',
        detail: 'Run `gisa monitor` in a terminal instead.',
        actions: [],
        healthy: false,
        banner: false,
      };
    default:
      return UNRECOGNIZED;
  }
}

function runningDetail(status: MonitorAgentStatusDto): string {
  const parts = [status.pid ? `PID ${status.pid}` : ''];
  if (status.last_scan) parts.push(`last scan ${status.last_scan}`);
  return parts.filter(Boolean).join(' · ');
}

export function actionLabel(action: MonitorAction, status: MonitorAgentStatusDto | null): string {
  if (action === 'start') return status?.state === 'failed' ? 'Retry' : 'Start';
  return action === 'stop' ? 'Stop' : 'Restart';
}

/**
 * An empty repository list only hints at a Full Disk Access problem once the
 * current monitor process has completed a scan. While it is starting (or
 * while nothing runs) an empty list means nothing.
 */
export function shouldSuggestFullDiskAccess(input: {
  status: MonitorAgentStatusDto | null;
  extensionEnabled: boolean;
  workspaceCount: number;
  repoCount: number;
}): boolean {
  return (
    input.status?.state === 'running' &&
    input.extensionEnabled &&
    input.workspaceCount > 0 &&
    input.repoCount === 0
  );
}

/**
 * Orders a fetched status against pushed events. The app subscribes first
 * and fetches second, so an event can arrive while the fetch is in flight;
 * the fetch result is then older and must not overwrite it.
 */
export function createStatusSequencer<T>() {
  let events = 0;
  return {
    beginFetch(): number {
      return events;
    },
    /** Returns the value to store, or `undefined` to keep the newer event. */
    acceptFetch(token: number, value: T): T | undefined {
      return token === events ? value : undefined;
    },
    acceptEvent(value: T): T {
      events += 1;
      return value;
    },
  };
}
