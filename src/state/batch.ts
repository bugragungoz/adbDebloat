/**
 * A batch from review to result. Nothing changes on the phone before the owner has seen the list
 * in the review dialog and pressed Start; the Rust side checks the safety rules again.
 */
import { signal } from '@preact/signals';
import { api } from '../api';
import type { Action, ItemResult, Session } from '../api/types';
import { errorMessage } from '../lib/errors';
import { loadPackages, ready } from './device';
import { loadHistory } from './history';
import { clearSelection } from './ui';

export interface Review {
  action: Action;
  packages: string[];
  /** Where the batch came from, for the dialog's wording. */
  undo?: Session;
}

export interface Run {
  action: Action;
  total: number;
  results: ItemResult[];
  session: Session | null;
  error: string | null;
}

export const review = signal<Review | null>(null);
export const run = signal<Run | null>(null);

export function openReview(action: Action, packages: string[], undo?: Session): void {
  run.value = null;
  review.value = { action, packages, undo };
}

export function closeReview(): void {
  review.value = null;
  run.value = null;
}

export async function startBatch(packages: string[]): Promise<void> {
  const r = review.value;
  const d = ready.value;
  if (!r || !d || !packages.length) return;
  run.value = { action: r.action, total: packages.length, results: [], session: null, error: null };
  const off = await api.onBatchProgress((p) => {
    const current = run.value;
    if (current) run.value = { ...current, results: [...current.results, p.item] };
  });
  try {
    const session = await api.apply(d.serial, r.action, packages);
    run.value = { ...run.value!, results: session.items, session };
    clearSelection();
  } catch (e) {
    run.value = { ...run.value!, error: errorMessage(e) };
  } finally {
    off();
    void loadPackages();
    void loadHistory();
  }
}

export function stopBatch(): void {
  void api.stopBatch();
}
