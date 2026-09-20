// Service status of the background monitor.
//
// Deliberately separate from the Finder data snapshot in `status.ts`: whether
// a monitor process runs and how fresh the badge data is are two questions.

import { writable } from 'svelte/store';
import {
  createStatusSequencer,
  type MonitorAction,
} from '../lib/monitorPresentation';
import {
  monitorStatus as fetchMonitorStatus,
  onMonitorAgentUpdated,
  restartMonitor,
  startMonitor,
  stopMonitor,
} from '../lib/tauri';
import type { MonitorAgentStatusDto } from '../lib/types';

export const monitorStatus = writable<MonitorAgentStatusDto | null>(null);
/** A lifecycle operation is running: duplicate actions are disabled. */
export const monitorBusy = writable(false);
export const monitorError = writable('');

const sequencer = createStatusSequencer<MonitorAgentStatusDto>();

/** Subscribe before fetching so no update can be missed. */
export async function subscribeMonitor(): Promise<() => void> {
  return onMonitorAgentUpdated((status) => {
    monitorStatus.set(sequencer.acceptEvent(status));
  });
}

export async function loadMonitorStatus(): Promise<void> {
  const token = sequencer.beginFetch();
  try {
    const fetched = sequencer.acceptFetch(token, await fetchMonitorStatus());
    if (fetched) monitorStatus.set(fetched);
  } catch (err) {
    monitorError.set(String(err));
  }
}

const OPERATIONS: Record<MonitorAction, () => Promise<MonitorAgentStatusDto>> = {
  start: startMonitor,
  stop: stopMonitor,
  restart: restartMonitor,
};

let inFlight = false;

/** Runs a lifecycle action. The returned status is used directly. */
export async function runMonitorAction(action: MonitorAction): Promise<void> {
  if (inFlight) return;
  inFlight = true;
  monitorBusy.set(true);
  monitorError.set('');
  try {
    monitorStatus.set(sequencer.acceptEvent(await OPERATIONS[action]()));
  } catch (err) {
    monitorError.set(String(err));
    await loadMonitorStatus();
  } finally {
    inFlight = false;
    monitorBusy.set(false);
  }
}
