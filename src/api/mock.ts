/**
 * A simulated phone for the plain browser: a Redmi 5 Plus on MIUI 12 with the packages such a phone
 * has, following the same safety rules as crates/adb-core/src/safety.rs. Nothing here touches a
 * real device. `?scenario=` picks a start: `noadb` (adb must be downloaded), `nophone`,
 * `unauthorized`, or the default, a phone ready to use.
 */
import type { Api } from '.';
import builtin from '../../crates/adb-core/data/packages.json';
import { AppError } from './invoke';
import type {
  Action,
  AdbTool,
  AppRow,
  BatchProgress,
  DeviceEntry,
  DeviceInfo,
  DownloadProgress,
  ItemResult,
  KnowledgeStatus,
  PackageState,
  Protection,
  Removal,
  Session,
  Settings,
  Status,
} from './types';

interface Known {
  label?: string;
  list: string;
  removal: Removal;
  description: string;
  dependencies?: string[];
  neededBy?: string[];
}

/** Packages adbDebloat's list does not describe, in the simulation's own words. */
const EXTRA: Record<string, Known> = {
  android: { label: 'Android System', list: 'aosp', removal: 'unsafe', description: 'The Android framework itself.' },
  'com.android.systemui': { label: 'System UI', list: 'aosp', removal: 'unsafe', description: 'The status bar, notifications and navigation.' },
  'com.android.settings': { label: 'Settings', list: 'aosp', removal: 'unsafe', description: 'The Settings app.' },
  'com.android.phone': { label: 'Phone Services', list: 'aosp', removal: 'expert', description: 'Calls and the SIM card.' },
  'com.android.providers.settings': { label: 'Settings Storage', list: 'aosp', removal: 'unsafe', description: 'Stores every system setting.' },
  'com.miui.securitycenter': { label: 'Security', list: 'oem', removal: 'expert', description: 'MIUI security, permissions and cleaner.' },
  'com.google.android.inputmethod.latin': { label: 'Gboard', list: 'google', removal: 'advanced', description: "Google's keyboard." },
  'com.android.providers.downloads': { label: 'Download Manager', list: 'aosp', removal: 'unsafe', description: 'Downloads files for apps and the Play Store.' },
  'com.android.bluetooth': { label: 'Bluetooth', list: 'aosp', removal: 'unsafe', description: 'Bluetooth connections.' },
  'com.android.camera': { label: 'Camera', list: 'oem', removal: 'advanced', description: "MIUI's camera app." },
  'com.android.deskclock': { label: 'Clock', list: 'aosp', removal: 'advanced', description: 'Clock, alarms and timers.' },
  'com.miui.miwallpaper': { label: 'Live wallpapers', list: 'oem', removal: 'recommended', description: "MIUI's live wallpapers." },
};

const KNOWN: Record<string, Known> = { ...(builtin as Record<string, Known>), ...EXTRA };

const CORE = new Set([
  'android',
  'com.android.systemui',
  'com.android.settings',
  'com.android.phone',
  'com.android.shell',
  'com.android.server.telecom',
  'com.android.providers.settings',
  'com.android.providers.telephony',
  'com.android.packageinstaller',
  'com.google.android.packageinstaller',
  'com.android.permissioncontroller',
  'com.google.android.permissioncontroller',
  'com.android.networkstack',
  'com.google.android.networkstack',
  'com.miui.securitycenter',
  'com.miui.system',
  'com.miui.rom',
]);

const SYSTEM_PACKAGES = [
  'android',
  'com.android.systemui',
  'com.android.settings',
  'com.android.phone',
  'com.android.providers.settings',
  'com.android.providers.downloads',
  'com.android.bluetooth',
  'com.android.camera',
  'com.android.deskclock',
  'com.android.chrome',
  'com.android.vending',
  'com.android.updater',
  'com.google.android.gms',
  'com.google.android.inputmethod.latin',
  'com.google.android.apps.tachyon',
  'com.google.android.videos',
  'com.google.android.apps.docs',
  'com.google.android.apps.photos',
  'com.google.android.apps.maps',
  'com.google.android.youtube',
  'com.google.android.apps.youtube.music',
  'com.google.android.gm',
  'com.google.android.feedback',
  'com.google.android.partnersetup',
  'com.google.android.onetimeinitializer',
  'com.google.android.marvin.talkback',
  'com.google.android.syncadapters.contacts',
  'com.google.android.syncadapters.calendar',
  'com.google.android.googlequicksearchbox',
  'com.google.android.webview',
  'com.miui.home',
  'com.miui.securitycenter',
  'com.miui.msa.global',
  'com.miui.systemAdSolution',
  'com.miui.analytics',
  'com.miui.daemon',
  'com.miui.hybrid',
  'com.miui.hybrid.accessory',
  'com.miui.bugreport',
  'com.xiaomi.mipicks',
  'com.miui.yellowpage',
  'com.miui.videoplayer',
  'com.miui.player',
  'com.mi.globalbrowser',
  'com.miui.weather2',
  'com.miui.cleanmaster',
  'com.miui.personalassistant',
  'com.miui.compass',
  'com.miui.cloudservice',
  'com.miui.cloudbackup',
  'com.xiaomi.gamecenter',
  'com.miui.notes',
  'com.miui.fm',
  'com.miui.screenrecorder',
  'com.miui.gallery',
  'com.miui.calculator',
  'com.miui.android.fashiongallery',
  'com.miui.miwallpaper',
  'com.mi.android.globalFileexplorer',
  'com.xiaomi.account',
  'com.miui.powerkeeper',
  'com.xiaomi.finddevice',
  'com.miui.mms',
  'com.facebook.appmanager',
  'com.facebook.services',
  'com.facebook.system',
  'com.netflix.partner.activation',
];
const USER_PACKAGES = ['com.spotify.music', 'com.whatsapp', 'org.telegram.messenger', 'com.instagram.android'];
const REFUSE_DISABLE = new Set(['com.miui.weather2', 'com.miui.securitycenter', 'com.miui.cleanmaster']);
const ADMINS = new Set(['com.xiaomi.finddevice']);

interface SimPackage {
  name: string;
  system: boolean;
  state: PackageState;
}

const params = typeof location !== 'undefined' ? new URLSearchParams(location.search) : new URLSearchParams();
const scenario = params.get('scenario') ?? 'ready';
const wait = (ms: number) => new Promise((r) => setTimeout(r, params.has('fast') ? 0 : ms));

const USB_SERIAL = '4b2a1c3d';
let adb: AdbTool | null = scenario === 'noadb' ? null : { source: 'managed', path: 'C:\\Users\\demo\\AppData\\Local\\com.bugragungoz.adbdebloat\\platform-tools\\adb.exe', version: '35.0.2' };
let usb: DeviceEntry | null =
  scenario === 'nophone' || scenario === 'noadb'
    ? null
    : {
        serial: USB_SERIAL,
        state: scenario === 'unauthorized' ? 'unauthorized' : 'ready',
        transport: 'usb',
        model: scenario === 'unauthorized' ? null : 'Redmi_5_Plus',
        device: 'vince',
        product: 'vince',
      };
let wifi: DeviceEntry | null = null;
// An unauthorized phone is allowed a few seconds after the page opens, as if the owner tapped Allow.
if (scenario === 'unauthorized') {
  setTimeout(() => {
    if (usb) usb = { ...usb, state: 'ready', model: 'Redmi_5_Plus' };
  }, params.has('fast') ? 300 : 4000);
}

let knowledge: KnowledgeStatus = { source: 'community', packages: 5381, fetchedAt: Date.now() - 2 * 24 * 3600 * 1000 };
let downloading = false;
let busy = false;
let stopRequested = false;

const phone: SimPackage[] = [
  ...SYSTEM_PACKAGES.map((name) => ({ name, system: true, state: 'installed' as PackageState })),
  ...USER_PACKAGES.map((name) => ({ name, system: false, state: 'installed' as PackageState })),
].sort((a, b) => a.name.localeCompare(b.name));
// A phone someone already cleaned a little.
for (const p of phone) {
  if (['com.miui.msa.global', 'com.miui.analytics', 'com.facebook.appmanager'].includes(p.name)) p.state = 'removed';
  if (p.name === 'com.google.android.apps.youtube.music') p.state = 'disabled';
}

const INFO: DeviceInfo = {
  serial: USB_SERIAL,
  name: 'Redmi 5 Plus',
  brand: 'xiaomi',
  manufacturer: 'Xiaomi',
  model: 'Redmi 5 Plus',
  codename: 'vince',
  android: '8.1.0',
  sdk: 27,
  skin: 'MIUI 12',
  build: 'OPM1.171019.019',
  battery: 76,
  launchers: ['com.miui.home'],
  keyboard: 'com.google.android.inputmethod.latin',
};

const SETTINGS_KEY = 'adbdebloat-demo-settings';
const DEFAULT_SETTINGS: Settings = { theme: 'system', language: 'system', expertMode: false, communityList: true, adbPath: '' };
let settingsCache: Settings = (() => {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    return raw ? { ...DEFAULT_SETTINGS, ...(JSON.parse(raw) as Partial<Settings>) } : DEFAULT_SETTINGS;
  } catch {
    return DEFAULT_SETTINGS;
  }
})();

let history: Session[] = [];
const batchListeners = new Set<(p: BatchProgress) => void>();
const downloadListeners = new Set<(p: DownloadProgress) => void>();

function devices(): DeviceEntry[] {
  return [usb, wifi].filter((d): d is DeviceEntry => d !== null);
}

function requireDevice(serial: string): DeviceEntry {
  if (!adb) throw new AppError('no_adb', '');
  const d = devices().find((x) => x.serial === serial);
  if (!d || d.state !== 'ready') throw new AppError('adb_failed', `adb: device '${serial}' not found`);
  return d;
}

function verdict(name: string, removal: Removal | null): { verdict: AppRow['verdict']; protection: Protection | null } {
  if (CORE.has(name)) return { verdict: 'protected', protection: 'core' };
  if (INFO.launchers.includes(name)) return { verdict: 'protected', protection: 'launcher' };
  if (INFO.keyboard === name) return { verdict: 'protected', protection: 'keyboard' };
  if (removal === 'unsafe') return { verdict: 'protected', protection: 'unsafe' };
  if (removal === 'expert' && !settingsCache.expertMode) return { verdict: 'needsExpert', protection: null };
  return { verdict: 'allowed', protection: null };
}

function row(p: SimPackage): AppRow {
  const k = KNOWN[p.name];
  const dir = p.system ? '/system/app' : '/data/app';
  return {
    name: p.name,
    label: k?.label ?? null,
    path: `${dir}/${p.name}/base.apk`,
    system: p.system,
    state: p.state,
    removal: k?.removal ?? null,
    list: k?.list ?? null,
    description: k?.description ?? null,
    dependencies: k?.dependencies ?? [],
    neededBy: k?.neededBy ?? [],
    ...verdict(p.name, k?.removal ?? null),
  };
}

function change(action: Action, p: SimPackage): ItemResult['failure'] | null {
  if (action === 'remove') {
    if (p.state === 'removed') return { code: 'nothing_to_do', detail: '' };
    if (ADMINS.has(p.name)) return { code: 'device_admin', detail: 'Failure [DELETE_FAILED_DEVICE_POLICY_MANAGER]' };
    p.state = 'removed';
  } else if (action === 'disable') {
    if (p.state !== 'installed') return { code: 'nothing_to_do', detail: '' };
    if (REFUSE_DISABLE.has(p.name)) {
      return { code: 'disable_refused', detail: `java.lang.SecurityException: Shell cannot change component state for ${p.name}/null to 3` };
    }
    p.state = 'disabled';
  } else {
    if (p.state === 'installed') return { code: 'nothing_to_do', detail: '' };
    p.state = 'installed';
  }
  return null;
}

export const mockApi: Api = {
  simulated: true,
  async getStatus(): Promise<Status> {
    return {
      adb,
      adbProblem: adb ? null : { code: 'missing', detail: '' },
      devices: adb ? devices() : [],
      knowledge,
      busy,
      downloading,
    };
  },
  async getDevice(serial) {
    const d = requireDevice(serial);
    await wait(150);
    return { ...INFO, serial: d.serial };
  },
  async getPackages(serial) {
    requireDevice(serial);
    await wait(250);
    return phone.map(row);
  },
  async apply(serial, action, packages) {
    const d = requireDevice(serial);
    if (busy) throw new AppError('busy', '');
    busy = true;
    stopRequested = false;
    const items: ItemResult[] = [];
    try {
      for (const [index, name] of packages.entries()) {
        await wait(140);
        const p = phone.find((x) => x.name === name);
        const k = KNOWN[name];
        const item: ItemResult = { package: name, before: p?.state ?? null, ok: false };
        if (k?.label) item.label = k.label;
        const v = verdict(name, k?.removal ?? null);
        if (stopRequested) item.failure = { code: 'no_answer', detail: '' };
        else if (!p) item.failure = { code: 'unknown', detail: '' };
        else if (action !== 'restore' && v.verdict === 'needsExpert') item.failure = { code: 'needs_expert', detail: '' };
        else if (action !== 'restore' && v.verdict === 'protected') item.failure = { code: 'protected', detail: '' };
        else {
          const failure = change(action, p);
          if (failure) item.failure = failure;
          else item.ok = true;
        }
        items.push(item);
        for (const l of batchListeners) l({ index, total: packages.length, item });
      }
    } finally {
      busy = false;
    }
    const at = Date.now();
    const session: Session = { id: String(at), at, device: { serial: d.serial, name: INFO.name, codename: INFO.codename }, action, items };
    history = [session, ...history];
    return session;
  },
  async stopBatch() {
    stopRequested = true;
  },
  async getHistory() {
    return history;
  },
  async pair(address, code) {
    await wait(600);
    if (!/^\d{6}$/.test(code.trim()) || !/^[\d.]+:\d+$/.test(address.trim())) throw new AppError('invalid', address);
    if (code.trim() !== '123456') throw new AppError('adb_failed', 'Failed: Wrong password or connection was dropped.');
    return `Successfully paired to ${address.trim()} [guid=adb-4b2a1c3d-AbCdEf]`;
  },
  async connect(address) {
    await wait(500);
    if (!/^[\d.]+:\d+$/.test(address.trim())) throw new AppError('invalid', address);
    wifi = { serial: address.trim(), state: 'ready', transport: 'wifi', model: 'Redmi_5_Plus', device: 'vince', product: 'vince' };
    return `connected to ${address.trim()}`;
  },
  async disconnect(serial) {
    if (wifi?.serial === serial) wifi = null;
  },
  async switchToWifi(serial) {
    requireDevice(serial);
    await wait(900);
    wifi = { serial: '192.168.1.23:5555', state: 'ready', transport: 'wifi', model: 'Redmi_5_Plus', device: 'vince', product: 'vince' };
    return wifi.serial;
  },
  async reboot(serial) {
    const d = requireDevice(serial);
    d.state = 'offline';
    setTimeout(() => (d.state = 'ready'), params.has('fast') ? 200 : 5000);
  },
  async downloadPlatformTools() {
    if (downloading) throw new AppError('busy', '');
    downloading = true;
    const total = 7_340_032;
    try {
      for (let done = 0; done < total; done += 917_504) {
        await wait(150);
        for (const l of downloadListeners) l({ done: Math.min(done, total), total });
      }
      adb = { source: 'managed', path: 'C:\\Users\\demo\\AppData\\Local\\com.bugragungoz.adbdebloat\\platform-tools\\adb.exe', version: '35.0.2' };
      return adb;
    } finally {
      downloading = false;
    }
  },
  async refreshKnowledge() {
    await wait(700);
    knowledge = { source: 'community', packages: 5381, fetchedAt: Date.now() };
    return knowledge;
  },
  async getSettings() {
    return settingsCache;
  },
  async setSettings(settings) {
    settingsCache = settings;
    if (!settings.communityList) knowledge = { source: 'builtin', packages: Object.keys(builtin).length, fetchedAt: null };
    else if (knowledge.source === 'builtin') knowledge = { source: 'community', packages: 5381, fetchedAt: Date.now() };
    try {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
    } catch {
      // Private windows may refuse storage; the setting then lasts until the page closes.
    }
    return settings;
  },
  async openUrl(url) {
    window.open(url, '_blank', 'noopener');
  },
  async onBatchProgress(callback) {
    batchListeners.add(callback);
    return () => batchListeners.delete(callback);
  },
  async onDownloadProgress(callback) {
    downloadListeners.add(callback);
    return () => downloadListeners.delete(callback);
  },
};
