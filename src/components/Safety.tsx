/**
 * How safe removing a package is, as a pill with a word and an icon (never color alone), and
 * what the safety rules say about it.
 */
import type { AppRow, Removal } from '../api/types';
import { t } from '../i18n';
import { Icon } from './Icon';

const TONE: Record<Removal, { tone: string; icon: string }> = {
  recommended: { tone: 'badge-ok', icon: 'check' },
  advanced: { tone: 'badge-info', icon: 'info' },
  expert: { tone: 'badge-warn', icon: 'alert' },
  unsafe: { tone: 'badge-warn', icon: 'lock' },
};

export function SafetyBadge({ removal }: { removal: Removal | null }) {
  if (!removal) {
    return <span class="badge">{t('safety.unknown')}</span>;
  }
  const { tone, icon } = TONE[removal];
  return (
    <span class={`badge ${tone}`} title={t(`safety.${removal}Hint`)}>
      <Icon name={icon} size={12} />
      {t(`safety.${removal}`)}
    </span>
  );
}

/** Why a package cannot be removed or disabled here; null when it can. */
export function lockReason(row: AppRow): string | null {
  if (row.verdict === 'protected') return t(`protection.${row.protection ?? 'core'}`);
  if (row.verdict === 'needsExpert') return t('protection.needsExpert');
  return null;
}

/** The name people know, else the package name. */
export function displayName(row: { label?: string | null; name: string }): string {
  return row.label || row.name;
}

/** The first sentence of a description, for the table. */
export function firstLine(text: string | null): string {
  if (!text) return '';
  const line = text.split('\n')[0]!.trim();
  const dot = line.search(/\.\s/);
  return dot > 0 ? line.slice(0, dot + 1) : line;
}

const LISTS = new Set(['google', 'oem', 'aosp', 'carrier', 'misc']);

/** Who ships a package: Google, the maker, Android itself, the carrier, someone else. */
export function listLabel(list: string): string {
  return LISTS.has(list) ? t(`list.${list}`) : list;
}
