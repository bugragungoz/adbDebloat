/** What the Rust side sends, in the shapes of src-tauri/src/commands.rs and crates/adb-core. */

export type DeviceState = 'ready' | 'unauthorized' | 'offline' | 'noPermission' | 'otherMode';
export type Transport = 'usb' | 'wifi' | 'emulator';

export interface DeviceEntry {
  serial: string;
  state: DeviceState;
  transport: Transport;
  model: string | null;
  device: string | null;
  product: string | null;
}

export interface DeviceInfo {
  serial: string;
  name: string;
  brand: string;
  manufacturer: string;
  model: string;
  codename: string;
  android: string;
  sdk: number;
  skin: string;
  build: string;
  battery: number | null;
  launchers: string[];
  keyboard: string | null;
}

export type AdbSource = 'custom' | 'managed' | 'path' | 'sdk';

export interface AdbTool {
  source: AdbSource;
  path: string;
  version: string;
}

export interface KnowledgeStatus {
  source: 'builtin' | 'community';
  packages: number;
  fetchedAt: number | null;
}

export interface Status {
  adb: AdbTool | null;
  adbProblem: { code: 'missing' | 'broken'; detail: string } | null;
  devices: DeviceEntry[];
  knowledge: KnowledgeStatus;
  busy: boolean;
  downloading: boolean;
}

export type PackageState = 'installed' | 'disabled' | 'removed';
export type Removal = 'recommended' | 'advanced' | 'expert' | 'unsafe';
export type Protection = 'core' | 'unsafe' | 'launcher' | 'keyboard';
export type Verdict = 'allowed' | 'needsExpert' | 'protected';

export interface AppRow {
  name: string;
  label: string | null;
  path: string | null;
  system: boolean;
  state: PackageState;
  removal: Removal | null;
  list: string | null;
  description: string | null;
  dependencies: string[];
  neededBy: string[];
  verdict: Verdict;
  protection: Protection | null;
}

export type Action = 'remove' | 'disable' | 'restore';

export type FailureCode =
  | 'device_admin'
  | 'disable_refused'
  | 'refused'
  | 'unknown'
  | 'nothing_to_do'
  | 'protected'
  | 'needs_expert'
  | 'no_answer'
  | 'other';

export interface ItemResult {
  package: string;
  label?: string;
  before: PackageState | null;
  ok: boolean;
  failure?: { code: FailureCode; detail: string };
}

export interface Session {
  id: string;
  at: number;
  device: { serial: string; name: string; codename: string };
  action: Action;
  items: ItemResult[];
}

export interface BatchProgress {
  index: number;
  total: number;
  item: ItemResult;
}

export interface DownloadProgress {
  done: number;
  total: number | null;
}

export interface Settings {
  theme: 'system' | 'dark' | 'light';
  language: 'system' | 'en' | 'tr';
  expertMode: boolean;
  communityList: boolean;
  adbPath: string;
}
