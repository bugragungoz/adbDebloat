/** Where the window is: the page, the Apps filters and selection, and the package in the panel. */
import { signal } from '@preact/signals';
import type { Removal } from '../api/types';

export type Page = 'phone' | 'apps' | 'history' | 'settings';
export type StateFilter = 'installed' | 'off' | 'all';
export type Level = Removal | 'unknown';

export const page = signal<Page>('phone');

export const search = signal('');
export const stateFilter = signal<StateFilter>('installed');
export const levels = signal<Set<Level>>(new Set(['recommended', 'advanced', 'expert', 'unsafe', 'unknown']));
export const systemOnly = signal(false);

export const selection = signal<Set<string>>(new Set());
export const openPackage = signal<string | null>(null);

export function toggleLevel(level: Level): void {
  const next = new Set(levels.value);
  if (next.has(level)) next.delete(level);
  else next.add(level);
  levels.value = next;
}

export function setSelected(names: string[], on: boolean): void {
  const next = new Set(selection.value);
  for (const n of names) {
    if (on) next.add(n);
    else next.delete(n);
  }
  selection.value = next;
}

export function clearSelection(): void {
  selection.value = new Set();
}
