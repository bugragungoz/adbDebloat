/**
 * The backend: the Rust commands inside the app, or a simulated phone when the page runs in a
 * plain browser (`npm run dev:web`, the UI tests and the screenshots).
 */
import type { Action, AdbTool, AppRow, BatchProgress, DeviceInfo, DownloadProgress, KnowledgeStatus, Session, Settings, Status } from './types';
import { call } from './invoke';
import { mockApi } from './mock';

export interface Api {
  readonly simulated: boolean;
  getStatus(): Promise<Status>;
  getDevice(serial: string): Promise<DeviceInfo>;
  getPackages(serial: string): Promise<AppRow[]>;
  apply(serial: string, action: Action, packages: string[]): Promise<Session>;
  stopBatch(): Promise<void>;
  getHistory(): Promise<Session[]>;
  pair(address: string, code: string): Promise<string>;
  connect(address: string): Promise<string>;
  disconnect(serial: string): Promise<void>;
  switchToWifi(serial: string): Promise<string>;
  reboot(serial: string): Promise<void>;
  downloadPlatformTools(): Promise<AdbTool>;
  refreshKnowledge(): Promise<KnowledgeStatus>;
  getSettings(): Promise<Settings>;
  setSettings(settings: Settings): Promise<Settings>;
  openUrl(url: string): Promise<void>;
  /** Calls back for each package of a running batch; returns an unsubscribe. */
  onBatchProgress(callback: (p: BatchProgress) => void): Promise<() => void>;
  onDownloadProgress(callback: (p: DownloadProgress) => void): Promise<() => void>;
}

async function listen<T>(event: string, callback: (payload: T) => void): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');
  return listen<T>(event, (e) => callback(e.payload));
}

const tauriApi: Api = {
  simulated: false,
  getStatus: () => call('get_status'),
  getDevice: (serial) => call('get_device', { serial }),
  getPackages: (serial) => call('get_packages', { serial }),
  apply: (serial, action, packages) => call('apply', { serial, action, packages }),
  stopBatch: () => call('stop_batch'),
  getHistory: () => call('get_history'),
  pair: (address, code) => call('pair', { address, code }),
  connect: (address) => call('connect', { address }),
  disconnect: (serial) => call('disconnect', { serial }),
  switchToWifi: (serial) => call('switch_to_wifi', { serial }),
  reboot: (serial) => call('reboot', { serial }),
  downloadPlatformTools: () => call('download_platform_tools'),
  refreshKnowledge: () => call('refresh_knowledge'),
  getSettings: () => call('get_settings'),
  setSettings: (settings) => call('set_settings', { settings }),
  async openUrl(url) {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  },
  onBatchProgress: (callback) => listen('batch-progress', callback),
  onDownloadProgress: (callback) => listen('download-progress', callback),
};

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
export const api: Api = inTauri ? tauriApi : mockApi;
