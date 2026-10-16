/**
 * The phone: adb's status every two seconds, the phone chosen, its information and its packages.
 * A phone that appears is read at once; one that goes away is forgotten.
 */
import { computed, signal } from '@preact/signals';
import { api } from '../api';
import type { AppRow, DeviceEntry, DeviceInfo, Status } from '../api/types';
import { errorMessage } from '../lib/errors';

export const status = signal<Status | null>(null);
export const statusError = signal<string | null>(null);
export const chosenSerial = signal<string | null>(null);
export const device = signal<DeviceInfo | null>(null);
export const packages = signal<AppRow[] | null>(null);
export const packagesError = signal<string | null>(null);
export const loadingPackages = signal(false);

export const devices = computed<DeviceEntry[]>(() => status.value?.devices ?? []);

/** The phone in use: the one chosen, else the first one that is ready, else the first one. */
export const current = computed<DeviceEntry | null>(() => {
  const list = devices.value;
  return list.find((d) => d.serial === chosenSerial.value) ?? list.find((d) => d.state === 'ready') ?? list[0] ?? null;
});

export const ready = computed(() => (current.value?.state === 'ready' ? current.value : null));

let loadedFor: string | null = null;

export async function loadPackages(): Promise<void> {
  const d = ready.value;
  if (!d) return;
  loadingPackages.value = true;
  try {
    packages.value = await api.getPackages(d.serial);
    packagesError.value = null;
  } catch (e) {
    packagesError.value = errorMessage(e);
  } finally {
    loadingPackages.value = false;
  }
}

async function loadDevice(serial: string): Promise<void> {
  loadedFor = serial;
  device.value = null;
  packages.value = null;
  packagesError.value = null;
  try {
    const info = await api.getDevice(serial);
    if (loadedFor !== serial) return;
    device.value = info;
    await loadPackages();
  } catch (e) {
    if (loadedFor === serial) packagesError.value = errorMessage(e);
  }
}

export async function refreshStatus(): Promise<void> {
  try {
    status.value = await api.getStatus();
    statusError.value = null;
  } catch (e) {
    statusError.value = errorMessage(e);
    return;
  }
  const d = ready.value;
  if (d && d.serial !== loadedFor) void loadDevice(d.serial);
  if (!d && loadedFor) {
    loadedFor = null;
    device.value = null;
    packages.value = null;
  }
}

export function choose(serial: string): void {
  chosenSerial.value = serial;
  void refreshStatus();
}

let timer: ReturnType<typeof setInterval> | null = null;
export function startPolling(): void {
  if (timer) return;
  void refreshStatus();
  timer = setInterval(() => void refreshStatus(), 2000);
}
